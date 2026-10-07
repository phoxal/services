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

## Independent tests and affected-package CI

Each package runs its actual deterministic functionality with `cargo test --locked`, without another participant implementation, supervisor, router, native engine, hardware or wall-clock sleeps.
Private runtime integration modules drive the real SDK Harness, generated adapters, typed admissions and logical scheduler; only external I/O is replaced explicitly before backend construction.
Public subprocess/transport and physical robot acceptance are separate host boundaries.

CI selects packages with `bash .github/scripts/affected-packages.sh --event pull_request --base BASE_SHA --head HEAD_SHA` for a PR merge-base comparison or `--event push` for a direct push range.
Use `--all` for a local full selection; workflow_dispatch runs the full suite.
Every top-level Cargo manifest is an independent package, including newly added packages.
Renames consider both paths, removed package roots conservatively select all surviving packages, and shared scripts/workflows/toolchain inputs select all.
Documentation-only changes skip Rust rebuilds, while test/fixture data and component model/assets select their owner regardless of file extension.
Selector and final-gate regressions run with Bash 3.2 or newer, Git and jq using `bash .github/scripts/test-ci.sh`.
Rust package jobs use the declared Rust 1.88 minimum, with Linux and macOS gamepad jobs where applicable.
The always-run CI checks result rejects selection failure, unexpected skipped/cancelled/failed package jobs and successful zero-test or ignored-only runs.
