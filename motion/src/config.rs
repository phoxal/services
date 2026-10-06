use std::collections::BTreeMap;

/// One calibrated motor shaft driving a wheel on a configured side.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, phoxal::Config)]
#[serde(deny_unknown_fields)]
pub struct WheelActuator {
    /// Logical drive side; downstream components are selected only by connections.
    pub side: WheelSide,
    /// Motor shaft rotations per wheel rotation.
    #[serde(default = "unit_ratio")]
    pub gear_ratio: f64,
    /// Motor direction producing positive forward wheel travel.
    #[serde(default = "positive_direction")]
    pub direction_sign: i8,
}

/// Supported logical wheel roles in a differential drive.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize, phoxal::Config,
)]
#[serde(rename_all = "snake_case")]
pub enum WheelSide {
    Left,
    Right,
}

/// Admitted robot-specific motion limits and complete actuator membership.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, phoxal::Config)]
#[serde(deny_unknown_fields)]
pub struct MotionConfig {
    /// Maximum absolute forward body velocity in metres per second.
    pub max_linear_mps: f64,
    /// Maximum absolute yaw body velocity in radians per second.
    pub max_angular_radps: f64,
    /// The implemented drive geometry and named actuator membership.
    pub drive: DriveModel,
}

/// Supported drive calculations; unimplemented models cannot be selected.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, phoxal::Config)]
#[serde(rename_all = "snake_case")]
pub enum DriveModel {
    /// Two independently commanded sides of calibrated wheel actuators.
    Differential(DifferentialDrive),
}

impl DriveModel {
    pub fn differential(&self) -> &DifferentialDrive {
        match self {
            Self::Differential(config) => config,
        }
    }

    #[cfg(test)]
    pub fn differential_mut(&mut self) -> &mut DifferentialDrive {
        match self {
            Self::Differential(config) => config,
        }
    }
}

/// Geometry and wheel calibration for the supported differential drive.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, phoxal::Config)]
#[serde(deny_unknown_fields)]
pub struct DifferentialDrive {
    /// Wheel rolling radius in metres.
    pub wheel_radius_m: f64,
    /// Distance between wheel contact lines in metres.
    pub track_width_m: f64,
    /// Logical wheel names and their side/calibration, with no component identity.
    pub wheels: BTreeMap<String, WheelActuator>,
}

const fn unit_ratio() -> f64 {
    1.0
}
const fn positive_direction() -> i8 {
    1
}

pub(super) fn validate_motion_config(config: &MotionConfig) -> phoxal::Result<()> {
    let drive = config.drive.differential();
    for (name, value) in [
        ("max_linear_mps", config.max_linear_mps),
        ("max_angular_radps", config.max_angular_radps),
        ("wheel_radius_m", drive.wheel_radius_m),
        ("track_width_m", drive.track_width_m),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(anyhow::anyhow!("{name} must be finite and positive"));
        }
    }
    let max_wheel_rate = (config.max_linear_mps
        + config.max_angular_radps * drive.track_width_m / 2.0)
        / drive.wheel_radius_m;
    for side in [WheelSide::Left, WheelSide::Right] {
        let count = drive
            .wheels
            .values()
            .filter(|wheel| wheel.side == side)
            .count();
        if !(1..=4).contains(&count) {
            return Err(anyhow::anyhow!("each drive side requires 1 to 4 wheels"));
        }
    }
    for (name, wheel) in &drive.wheels {
        phoxal::artifact::bundle::validate_segment(name, "logical wheel name")
            .map_err(|error| anyhow::anyhow!(error))?;
        phoxal::artifact::bundle::validate_segment(
            &format!("{name}_actuator"),
            "logical wheel port",
        )
        .map_err(|error| anyhow::anyhow!(error))?;
        if !matches!(wheel.direction_sign, -1 | 1)
            || !wheel.gear_ratio.is_finite()
            || wheel.gear_ratio <= 0.0
        {
            return Err(anyhow::anyhow!(
                "wheel gearing must be finite and positive and direction must be -1 or 1"
            ));
        }
        if !(max_wheel_rate * wheel.gear_ratio).is_finite() {
            return Err(anyhow::anyhow!(
                "motion limits overflow motor angular velocity"
            ));
        }
    }
    Ok(())
}
