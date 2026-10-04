# World service

The World executable owns bounded localization belief, coherent revisions, immutable map-window reads, and explicit unavailable results.
Its current free-space fixture does not claim SLAM, perception, map persistence, or mature spatial planning.
Its Rust contract in `src/contract.rs` declares its endpoints and specialized payloads.
The robot selects its exact package version in `robot.yaml`; preparation extracts the compiled contract without a World library dependency.

The live transport proof launches the real World binary and belongs to explicit host acceptance.
Run it with `cargo test --features host-acceptance --test runtime_transport`.
Ordinary CI runs the deterministic unit suite and compiles the host assertions with strict Clippy.
