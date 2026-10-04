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

## Publication

Each standalone package owns its release-plz configuration and committed application lockfile.
The repository runs a small package matrix rather than sharing a workspace dependency selection.
Review standard version/changelog PRs, then publish tested revisions to crates.io through release-plz.
Package versions are independent; compatibility follows the interfaces each participant serves.
Owners must be available publicly before dependent package locks and releases are accepted.
