# Full-stack runtime qualification

This standalone robot fixture owns service composition qualification beyond the minimal public rover.
It retains Kinematics, World, Navigation, Safety, Motion, range sensing, and generated brain conversions.
Service sources select the owning packages in this checkout; component and supervisor sources use explicit pinned Git revisions.
The measured Safety freshness budget, service cadence, wheel qualification, movement, and stop assertions remain unchanged.
Native scenarios execute the authored composition through the real supervisor and user-managed MuJoCo.
Motion mission inputs are deliberately external in this fixture, so scenario stimuli are not competing with the brain's withdrawn mission projections.
Protective and odometry connections remain authored and required.
The fixture is an acceptance input, not a product or an ordinary Rust test suite.

Declare and execute `scenarios/full_stack.rs` as the `full-stack` Cargo example with `test = false`:

```sh
cargo phoxal scenario scenarios/full_stack.rs
```

Ordinary `cargo test` does not run the native scenario or require a command-scoped host.
