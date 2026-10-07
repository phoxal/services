//! Differential-drive body twist to motor shaft velocity, in SI units.
use crate::config::MotionConfig;
use crate::config::WheelSide;
use phoxal::contracts::component::actuator::Control;
use phoxal::contracts::robotics::MotionSetpoint;

/// Private invocation-local calculation, not a wire-addressed actuator message.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct WheelCommands {
    pub targets: Vec<WheelCommand>,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct WheelCommand {
    pub wheel_name: String,
    pub control: Option<Control>,
}

pub(super) fn stopped_setpoint(config: &MotionConfig) -> WheelCommands {
    setpoint_from_twist(0.0, 0.0, config)
}

pub(super) fn setpoint_from_intent(
    intent: &MotionSetpoint,
    config: &MotionConfig,
) -> WheelCommands {
    let linear = intent
        .linear_x_mps
        .clamp(-config.max_linear_mps, config.max_linear_mps);
    let angular = intent
        .angular_z_radps
        .clamp(-config.max_angular_radps, config.max_angular_radps);
    setpoint_from_twist(linear, angular, config)
}

pub(super) fn setpoint_from_twist(
    linear: f64,
    angular: f64,
    config: &MotionConfig,
) -> WheelCommands {
    let drive = config.drive.differential();
    let mut targets = Vec::with_capacity(drive.wheels.len());
    for (name, wheel) in &drive.wheels {
        let side = match wheel.side {
            WheelSide::Left => -1.0,
            WheelSide::Right => 1.0,
        };
        let wheel_rate =
            (linear + side * angular * drive.track_width_m / 2.0) / drive.wheel_radius_m;
        targets.push(WheelCommand {
            wheel_name: name.clone(),
            control: Some(Control::VelocityRadps(
                wheel_rate * wheel.gear_ratio * f64::from(wheel.direction_sign),
            )),
        });
    }
    WheelCommands { targets }
}
