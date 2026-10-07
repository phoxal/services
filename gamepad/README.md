# Gamepad service

This standalone Git/local-only package samples OS controller state at each accepted 20 ms runtime invocation and publishes the shared SDK body-frame MotionSetpoint with a 100 ms logical lease.
Native deterministic rover driving is qualified separately from partial physical Stadia USB and wrapped desktop acceptance.
Observed physical Manual authority for gamepad, forward/nonzero rover movement and later commanded-zero/stationarity do not complete the physical campaign.
Directed backward/turn/deadman-release, unplug while moving, reconnect-held/fresh neutral reengagement and held desktop interaction/cleanup remain open.
The observed zero/disarm cause is not yet tied to a directed L1 release, and backward movement is human-reported only.
Motion owns arm/disarm, authority, arbitration and robot safety limits.
The service has private typed expectations for Motion's Arm and Disarm contracts, with no dependency on the Motion implementation package.

## Input and configuration

Pair controllers through the operating system.
The package uses [gilrs 0.11.2](https://docs.rs/crate/gilrs/0.11.2), which supports native macOS and Linux input and hotplug.
[Gilrs::next_event](https://docs.rs/gilrs/0.11.2/gilrs/struct.Gilrs.html#method.next_event) is nonblocking; this service never calls next_event_blocking.
Linux builds need pkg-config and libudev development metadata, commonly pkg-config and libudev-dev on Debian-derived hosts.
The running user also needs access to the appropriate /dev/input/event devices.
No Bluetooth or pairing implementation is included.
The library exposes initialization errors, disconnect events and optional pending events, but no per-poll backend-health Result.
The service reports failures it can observe rather than claiming to detect every native backend failure.

```yaml
device: {mode: auto}
linear: {axis: left_y, deadzone: 0.12, invert: false, scale: 0.5}
angular: {axis: right_x, deadzone: 0.12, invert: true, scale: 1.5}
deadman: left_bumper
```

Device modes are auto with no selector, name with one exact OS name, or index with one current host index.
Auto requires exactly one suitable controller; ambiguous names or devices are refused.
A host index is not a durable device identity.
Axes are left_x, left_y, right_x and right_y; buttons are left_bumper, right_bumper, south and east.
Gilrs normalizes stick Y to positive upwards on both supported platforms.
The default right-X inversion converts rightward input to clockwise, negative body yaw.
The deadzone is continuous and rescales the remaining range to the configured maximum.
Scales are finite positive maxima in m/s and rad/s, with Motion applying robot-owned limits afterward.
Default gilrs filters are disabled so the authored deadzone is applied once.
Polling drains at most 256 events plus one overflow lookahead and inspects at most 16 controllers.
An overflow withdraws intent and requires release after the backlog clears.
One controller incarnation stays selected throughout an engagement.

## Authority transitions

Startup, reset and reconnect require an observed release followed by a fresh neutral-stick deadman press.
The service first publishes a neutral leased intent, waits a later invocation, then requests Manual through arm_motion.
It keeps publishing neutral while arming and publishes stick motion only after an Accepted response while the controller remains valid and held.
Release, disconnect, unavailable mapping, invalid axis data and observed backend loss withdraw intent and request cleanup when authority may exist.
Idle input never repeatedly disarms Motion.
A late successful arm after release triggers cleanup again if an earlier disarm may have preceded that arm.
Definite refusal requires fresh release and press before retry.
Uncertain arm/disarm outcomes latch Fault, withdraw intent and require execution stop/reset rather than automatic rearming.
The logical call timeout is 500 ms; retiring a local ticket is not remote-effect cancellation.

Pause is a pure controlled-time freeze, preserving the world, runtime state, admitted commands, authority and logical leases.
A held connected controller resumes without Pause-specific rearming.
OS changes while paused become visible at the next normal 20 ms invocation and ordinary Motion admission; a retained command may act during that bounded delay, including a nondue first resumed native boundary.
Step advances one normal complete boundary and never forces an extra input poll.
Sampled release/disconnect still withdraws, and actual reconnect/reset still requires the normal safe engagement sequence.
Motion compares the authenticated caller instance with the leased producer instance, preserving rejection of another owner despite differing caller endpoint spelling.
No suspension generation, wall-clock lease expiry or Pause-specific automatic disarm is used.

## Qualification

MotionSetpoint is provided by the published Phoxal SDK 0.71.0.
The application lock resolves verified registry SDK archives; service distribution remains Git/local-only.
Select a local source for development or a qualified full Git revision for robot composition.

```sh
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --check
cargo doc --locked --no-deps
```

Test input injection exists only under cfg(test), without a production fake backend or environment switch.
Runtime Harness tests exercise the actual generated runtime owner, calls, accepted publications and reset fencing.
They do not by themselves qualify native supervisor pause/resume ordering or a physical controller.
The package is publish=false and distributed only through qualified Git or local source acquisition.

A harness-free native_process Cargo test target reuses these real private modules for explicit native qualification.
Only cfg(test) enables its bounded file-backed input adapter and diagnostics; production has no fake backend flag or environment switch.
The native worker qualification retains actual accepted/delivery/native-control evidence and is independent of a physical controller.

## Native Linux qualification

The current package and Motion pass native Linux aarch64 qualification in the local Docker Desktop Linux VM using Rust 1.88.0, matching their declared minimum Rust version.
The official image is rust:1.88-bookworm at digest sha256:af306cfa71d987911a781c37b59d7d67d934f49684058f96cf72079c3626bfe0.
The compiler host and both release ELF executables are aarch64-unknown-linux-gnu, with no cross compilation or emulation.
The disposable container receives only copied gamepad, Motion and local SDK/build/macros sources, without host credentials, device mounts or privileged access.
The copied workspace lists those three SDK crates and their independent schema-proof fixture; proof lockfiles are generated there, leaving application locks and robot preparation intact.

Inside that isolated source tree, /proof/overlay.toml patches phoxal, phoxal-build and phoxal-macros to their copied local paths.
These are local unpublished owner checks, not evidence for public SDK archives or participant Git pins.
The commands below run inside the container, with the same test/build/lint sequence repeated from /proof/services/motion.

```sh
apt-get update
apt-get install -y --no-install-recommends pkg-config libudev-dev
rustup component add clippy rustfmt
export CARGO_TARGET_DIR=/proof/target CARGO_BUILD_JOBS=6
cd /proof/services/gamepad
cargo test --config /proof/overlay.toml
cargo test --config /proof/overlay.toml host::tests::native_backend_snapshot -- --ignored --nocapture
cargo fmt --check
cargo clippy --config /proof/overlay.toml --all-targets -- -D warnings
cargo build --config /proof/overlay.toml --release
RUSTDOCFLAGS="-D warnings" cargo doc --config /proof/overlay.toml --no-deps
```

The image already supplies GCC 12.2.0 and pkg-config 1.8.1; libudev-dev and libudev1 are Debian 252.39-1~deb12u2 in this qualification.
All 13 gamepad tests and 21 Motion tests pass, including accepted runtime publication, reset fencing, timeout handling and same-instance arming.
The separately requested native gilrs backend test initializes and polls successfully with no devices and no fault.
Compiled release contracts retain the 20 ms period, 100 ms intent lease, shared phoxal.robotics.v1.MotionSetpoint and ordinary Motion Arm/Disarm expectations.
Native Linux device input remains unqualified: this container has neither /dev/input nor /run/udev, and no broader permissions were granted.
This proof does not qualify a physical controller, Linux simulator graphics, desktop gestures or public owner delivery.

The framework extraction fixture also provides an explicit host qualification driver for source-built participant artifacts.
It decodes all schema frames and assembles the full descriptor pool, including incidental cross-crate records, without executing the inspected binaries or filtering missing dependencies.

```sh
cd /proof/framework
PHOXAL_SCHEMA_PROOF_ARTIFACTS=/proof/target/release/phoxal-service-gamepad:/proof/target/release/phoxal-service-motion cargo test -p phoxal-schema-proof-fixture --release --test extraction external_runtime_frames_retain_complete_cross_crate_schema_closures -- --ignored --nocapture
```

## Explicit runtime qualification fixtures

`cargo test` drives the real generated runtime adapter through the SDK Harness with an explicitly supplied fixture input dependency.
The production host initializes only gilrs; neither the host nor runtime reads test environment variables, input files, or trace destinations.
The runtime's private `Input` boundary supplies one bounded OS frame per normal accepted invocation; configuration and all control, call, publication and acceptance behavior remain shared.
The fixture delegates dispatch, reset, endpoint encoding and accepted/discarded ownership to the generated adapter.
No alternate control state machine or scheduler implements the test runtime.

The harness-free `native_process` target is an explicit qualification executable using the same private runtime modules and compiled contract.
Its standard supervisor `--bundle-root` argument identifies a fixture-owned `gamepad-fixture.input` file inside that disposable bundle directory.
Only `tests/support/file_input.rs` interprets that bounded file, with released, held, drive, disconnected, release_repress, reconnected_held and reconnected_released fixture states.
There is no service configuration option, environment selector, or production fake backend.
The simulator's existing test-owned external-source driver orchestrates the file and typed worker commands and captures actual boundary products, delivery cuts and native actuator receipts.
Motion is the ordinary production executable, with no inline runtime trace instrumentation.
Historical trace logs qualify their historical source revision only.
The current native proof runner and final artifact hashes must be retained alongside each qualification result.

After removing inline test instrumentation and correcting fixture initialization before native backend construction, the current source was freshly qualified natively on Linux aarch64 with Rust 1.88.0 in the pinned image above.
All thirteen gamepad tests, strict Clippy/format/docs, optimized build and no-device gilrs startup pass.
Earlier eleven-test logs qualify their historical revisions and are retained separately.
Strict extraction of actual gamepad and Motion release ELFs retains complete descriptor closures, and compiled runtime/configuration records are unchanged.
Input fixtures use explicit test-owned dependencies; the normal production backend and source package remain the qualified implementation.
This build qualification still does not establish physical Linux controller input.

Fixture initialization and reset construct the supplied input before any native OS backend construction.
The private shared constructor validates the same typed Config and initializes the same state used by production.
The fixture resets and delegates the real generated adapter, including projection and accepted/discarded behavior.
An explicit Linux process observer protects that boundary without participant hooks:

```sh
python3 tests/input_isolation.py /path/to/unit-test-binary /tmp/gamepad-input-isolation.trace
```

This invokes strace around only the focused initialization/reset test and rejects native AF_NETLINK setup.
The original adapter-init-then-replace fixture reproduces that failure, while corrected initialization and reset make zero native netlink socket attempts.
The optional standalone tests/fixtures/public_observer program uses the normal public SDK session client and actual Motion status contract for read-only physical qualification.
Its explicit endpoint and bounded duration arguments belong only to that test-owned observer; it issues no arm, disarm, setpoint or simulator-control operations.
It reports admitted mode, selected owner and stopped-state transitions and rejects observation gaps/errors.
It is independent of the production participant configuration and input backend.
