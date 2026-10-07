# Phoxal services

Official Phoxal services, each with its own executable, contract, and package tests.
The framework SDK owns shared robotics vocabulary; service-specific contracts stay beside their service implementation.
Linux and macOS are supported.
Windows and other operating systems are unsupported and unqualified.

Each top-level package is independent and owns its dependency choices and committed application lockfile.
There is no repository-wide Cargo workspace or lockfile.
Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt -- --check` from the package directory.
CI adds `--locked` to verify that the committed lockfile is current.
Each executable has a local-input-only `build.rs` and one `phoxal::api!()` attachment.
Robot projects select these packages through `robot.yaml` and use `cargo phoxal prepare` before their ordinary build and test workflow.
Rust dependencies resolve from crates.io; authored participant selections use local paths or full pinned Git revisions.

## Distribution

Official participants are distributed through local paths or full pinned Git revisions, with `publish = false` in every package manifest.
This convention also applies to the gamepad service and future participants.
The [gamepad package](gamepad/README.md) has native deterministic qualification with pure-freeze Pause semantics and partial observed physical driving and desktop control/layout acceptance.
Its README records the remaining directed controller, held-gesture and Linux device-input gates.
Cargo can still resolve framework SDK and other Rust dependencies from crates.io.
Each package retains its own committed application lockfile, executable, contract, and resources.
Qualify the owner revision before updating robot Git pins, then rerun preparation, compiled checks, and runtime acceptance.
There is no crates.io publication or release-plz workflow for these participant packages.
