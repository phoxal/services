//! Standalone Linux observer for the real fixture initialization/reset test.
use std::{env, error::Error, fs, io, process::Command};

const FOCUSED_TEST: &str = "runtime::fixture::initialization_tests::fixture_initialization_and_reset_use_only_the_explicit_input_factory";

fn require_focused_pass(output: &str) -> Result<(), io::Error> {
    let expected = format!("test {FOCUSED_TEST} ... ok");
    let mut lines = output.lines();
    if !lines.any(|line| line == expected)
        || !output
            .lines()
            .any(|line| line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;"))
    {
        return Err(io::Error::other(
            "the intended initialization/reset test did not execute exactly once and pass",
        ));
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let binary = args.next().ok_or("expected test binary and trace path")?;
    let trace = args.next().ok_or("expected trace path")?;
    if args.next().is_some() {
        return Err("expected exactly test binary and trace path".into());
    }
    let result = Command::new("strace")
        .args(["-f", "-e", "trace=socket", "-o"])
        .arg(&trace)
        .arg(fs::canonicalize(binary)?)
        .args(["--exact", FOCUSED_TEST, "--nocapture"])
        .output()?;
    let stdout = String::from_utf8(result.stdout)?;
    print!("{stdout}");
    eprint!("{}", String::from_utf8_lossy(&result.stderr));
    if !result.status.success() {
        return Err(format!("focused test process failed: {}", result.status).into());
    }
    require_focused_pass(&stdout)?;
    if fs::read_to_string(trace)?.contains("AF_NETLINK") {
        return Err("fixture initialized the native udev gamepad backend".into());
    }
    println!(
        "one focused initialization/reset test passed with zero native netlink socket attempts"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_the_exact_successful_test() {
        let output = format!(
            "test {FOCUSED_TEST} ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored;\n"
        );
        assert!(require_focused_pass(&output).is_ok());
        assert!(require_focused_pass(&output.replace(FOCUSED_TEST, "wrong_test")).is_err());
    }

    #[test]
    fn rejects_zero_ignored_failed_and_multiple_tests() {
        for summary in [
            "ok. 0 passed; 0 failed; 0 ignored;",
            "ok. 0 passed; 0 failed; 1 ignored;",
            "FAILED. 0 passed; 1 failed; 0 ignored;",
            "ok. 2 passed; 0 failed; 0 ignored;",
        ] {
            let output = format!("test {FOCUSED_TEST} ... ok\ntest result: {summary}\n");
            assert!(require_focused_pass(&output).is_err());
        }
    }
}
