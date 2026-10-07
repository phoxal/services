# Navigation service

The Navigation executable owns bounded goal processing and publishes its state and terminal outcomes.
It consumes standard odometry and its own private map expectation through the robot's explicit graph connections.
Its Rust contract in `src/contract.rs` declares its endpoints, private `MapState` input, and specialized exported payloads.
The robot adapts World's `WorldRevision` to `MapState` in ordinary Rust conversion code.
Selecting the package in `robot.yaml` makes `api::navigation` available to that robot without adding a Navigation Cargo dependency.

Navigation does not claim a measured occupancy map or physical obstacle avoidance from generated types alone.

## Deterministic package qualification

Run `cargo test --locked`, `cargo fmt --check`, and `cargo clippy --locked --all-targets -- -D warnings` from this package.
Tests drive actual ordered Start/Cancel/status calls, map revision updates and original capture expiry, active-goal unavailability, typed refusal, single terminal events, real bounded terminal retention/eviction and reset.
Map inputs use the private expected payload, without a World implementation or robot conversion fixture.
This implementation reports bounded planner state and goal outcomes; these tests do not claim an implemented autonomous motor-control producer.
