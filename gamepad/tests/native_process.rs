//! Test-only executable reusing the actual private runtime and compiled contract.
// Included unit-test helpers are unused in this harness-free process target.
#![allow(dead_code, unused_imports)]
phoxal::api!();
#[path = "../src/config.rs"]
mod config;
#[path = "../src/contract.rs"]
mod contract;
#[path = "../src/control.rs"]
mod control;
#[path = "support/file_input.rs"]
mod file_input;
#[path = "../src/host.rs"]
mod host;
#[path = "../src/runtime.rs"]
mod runtime;
fn main() -> phoxal::Result<()> {
    if !std::env::args().any(|arg| arg == "--instance-id") {
        println!("native_process is a qualification fixture, not an ordinary test driver");
        return Ok(());
    }
    let launch = phoxal::runtime::runner::RuntimeLaunch::parse()?;
    runtime::fixture::launch(move || {
        Box::new(file_input::FileInput {
            path: launch.bundle_root.join("gamepad-fixture.input"),
            previous: String::new(),
            incarnation: 1,
        })
    })
}
