# World service

The World executable owns bounded localization belief, coherent revisions, immutable map-window reads, and explicit unavailable results.
Its grid contains Unknown occupancy because localization alone does not establish traversable free space.
It does not claim SLAM, perception, map persistence, or mature spatial planning.
Its Rust contract in `src/contract.rs` declares its endpoints and specialized payloads.
The robot selects its exact package version in `robot.yaml`; preparation extracts the compiled contract without a World library dependency.


## Deterministic package qualification

Run `cargo test --locked`, `cargo fmt --check`, and `cargo clippy --locked --all-targets -- -D warnings` from this package.
Tests admit typed odometry and actual generated window calls, including current/history/evicted reads, out-of-bounds refusal, unknown occupancy, same-invocation handler/step revision coherence, cadence, capture freshness and reset.
The complete returned grid must fit the encoded window response contract even for nonzero interior request coordinates and the maximum revision.
Initialization rejects oversized grids, nonrepresentable spatial extents and history capacities outside 1 through 256.
The historical live transport test is retired from this participant; sockets and process scheduling are not ordinary package-test prerequisites.
