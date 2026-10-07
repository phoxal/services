# Motion service

This executable owns final actuator intent, arm/disarm state, and emergency handling.
Its Rust contract in `src/contract.rs` declares its endpoints and specialized payloads.
Manual and autonomous inputs consume `phoxal::contracts::robotics::MotionSetpoint` in the shared `phoxal.robotics.v1` namespace.
Its fields are body-frame forward velocity `linear_x_mps` and counter-clockwise yaw rate `angular_z_radps`.
Producers depend on that SDK vocabulary, while Motion owns arming, authority, limits, modes, status, emergency handling, and arbitration.
Shared actuation records also come from `phoxal::contracts`.
Configuration, inputs, outputs, arbitration, and drive calculations live in separate executable modules.

Select `drive.differential` and configure a `wheels` map of logical names.
Each wheel selects its `side` (`left` or `right`), motor-to-wheel `gear_ratio`, and `direction_sign`.
Configure one to four wheels per side.
A wheel named `front_left` exposes the typed leased output `motion.front_left_actuator`.
Connect it to a component's actuator input in `robot.yaml`; configuration never names downstream components or motor targets.
Every invocation produces the complete configured wheel set, including stopped scalar commands.
The shared `wheel_radius_m` and explicit calibration `track_width_m` are metres.

```yaml
drive:
  differential:
    wheel_radius_m: 0.11
    track_width_m: 0.52
    wheels:
      front_left: {side: left, direction_sign: 1}
      front_right: {side: right, direction_sign: -1}
```

```yaml
robot:
  components:
    front_left_drive:
      driver:
        bindings:
          actuator: motion.front_left_actuator
    front_right_drive:
      driver:
        bindings:
          actuator: motion.front_right_actuator
```

The runtime starts disarmed and requires an explicit arm command and matching leased intent ownership.
Basic drive needs no Safety or odometry service.
When the authored graph connects `safety` or `measurements`, that input becomes required and must supply valid fresh evidence before arming or continued movement.
Missing, expired, invalid, or unavailable evidence stops Motion; reset preserves the connection requirement.
Both derived inputs retain fresh original capture evidence; a new publication timestamp cannot make stale sensor data usable.
Unconnected odometry does not create measured motion; Motion publishes commanded actuator intent and its control status.
Motor shaft rates apply each wheel's gearing and direction exactly once after converting the bounded body twist to wheel velocity.

The drive model is a typed enum; only Differential is implemented.
Logical wheel names determine ports; authored consumer bindings alone determine destinations.
Track width is the distance between wheel contact lines, not mounting sites.
The current configuration requires explicit wheel radius and track-width calibration.
Model-derived geometry is deferred; hardware builds and Motion do not require native model loading.

Arm calls use the SDK-authenticated caller instance as the authority owner, matching the admitted intent producer instance even when the call arrives through a generated endpoint.
Different instances cannot arm against another producer's leased intent.
Simulation Pause freezes admitted authority and logical leases; normal scheduling on resume determines subsequent input admission and withdrawal.

Runtime qualification uses ordinary typed contract outputs and supervisor/native boundary evidence.
The production runtime contains no test environment selectors or trace-file I/O.
External gamepad qualification runs this ordinary executable rather than a diagnostic runtime variant.

## Deterministic package qualification

Run `cargo test --locked`, `cargo fmt --check`, and `cargo clippy --locked --all-targets -- -D warnings` from this package.
Tests drive actual typed arm/disarm/emergency admissions, authenticated intent ownership, single-source logical leases, invalid intent and protective expiry, configured limits and every calibrated named wheel output.
Cadence tests distinguish bootstrap status from step-only actuation, and reset returns authority to Disarmed with no retained command.
Optional connected protective evidence is separately validated by the owning input-facts tests; these tests do not establish physical motor writes or native robot dynamics.
