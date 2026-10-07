# Safety service

The Safety executable owns bounded assessment of measured range and world evidence and publishes expiring Motion constraints.
It does not own emergency authority, final actuation, hardware drivers, or a map algorithm.
Its Rust contract in `src/contract.rs` declares its endpoints and specialized payloads.
The robot selects its exact package version in `robot.yaml`; preparation extracts the compiled contract without a Safety library dependency.

## Deterministic package qualification

Run `cargo test --locked`, `cargo fmt --check`, and `cargo clippy --locked --all-targets -- -D warnings` from this package.
Tests admit typed world, revision, motion and range captures, check required-source availability, stop/proximity/ground envelopes, original-capture-bounded constraint expiry and invalid replacement.
Reset discards retained sensor evidence, and nondue intervals do not renew constraints.
No World, Motion or range-driver implementation is imported.
