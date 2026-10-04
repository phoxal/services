//! The world estimation service executable: configuration, the private
//! payload vocabulary and endpoint contract, validation, and the runtime.

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
compile_error!("Phoxal supports Linux and macOS only");

phoxal::api!();

mod config;
mod contract;
mod runtime;
mod validation;

fn main() -> phoxal::Result<()> {
    phoxal::runtime::run::<runtime::World>()
}
