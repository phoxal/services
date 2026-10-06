//! Rover composition and mission policy, launched through the canonical runtime.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
compile_error!("Phoxal supports Linux and macOS only");

phoxal::api!();

mod conversions;
mod runtime;

fn main() -> phoxal::Result<()> {
    phoxal::runtime::run::<runtime::Brain>()
}
