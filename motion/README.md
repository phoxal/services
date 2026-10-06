# Motion service

This executable owns final actuator intent, arm/disarm state, and emergency handling.
Its Rust contract in `src/contract.rs` declares its endpoints and specialized payloads.
Shared actuation records come from `phoxal::contracts`.
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
connections:
  - from: motion.front_left_actuator
    to: front_left_drive.actuator
  - from: motion.front_right_actuator
    to: front_right_drive.actuator
```

The runtime starts disarmed and requires an explicit arm command and matching leased intent ownership.
Basic drive needs no Safety or odometry service.
When the authored graph connects `safety` or `measurements`, that input becomes required and must supply valid fresh evidence before arming or continued movement.
Missing, expired, invalid, or unavailable evidence stops Motion; reset preserves the connection requirement.
Both derived inputs retain fresh original capture evidence; a new publication timestamp cannot make stale sensor data usable.
Unconnected odometry does not create measured motion; Motion publishes commanded actuator intent and its control status.
Motor shaft rates apply each wheel's gearing and direction exactly once after converting the bounded body twist to wheel velocity.

The drive model is a typed enum; only Differential is implemented.
Logical wheel names determine ports; authored connections alone determine destinations.
Track width is the distance between wheel contact lines, not mounting sites.
The current configuration requires explicit wheel radius and track-width calibration.
Model-derived geometry is deferred; hardware builds and Motion do not require native model loading.
