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
Published package dependencies use the Phoxal registry; no sibling checkout is required.

## Publication

Review and merge package version changes normally before publication.
Dispatch the publication workflow on the approved revision, selecting one package and an independently released publication-tool version.
The workflow verifies its archive and submits it for registry review; a pending registry review is not a published release.
Packages retain independent versions, and compatibility follows the interfaces consumed by each operation.

## Current development boundary

The implementation requires framework SDK/build/macros `0.0.0-dev.8`.
That owner release is not yet in the public registry; current local qualification uses an explicit owner overlay.
Registry-backed fresh-checkout builds and installable releases remain blocked until publication and Cargo-generated consumer lockfiles are reconciled against it.
CI attempts the normal registry-backed build and reports this failure rather than substituting an unpublished source checkout.
