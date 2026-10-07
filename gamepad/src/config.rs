use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, phoxal::Config)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub device: DeviceSelection,
    #[serde(default)]
    pub linear: AxisConfig,
    #[serde(default = "angular_default")]
    pub angular: AxisConfig,
    #[serde(default)]
    pub deadman: Button,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, phoxal::Config)]
#[serde(deny_unknown_fields)]
pub struct DeviceSelection {
    #[serde(default)]
    pub mode: DeviceMode,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub index: Option<u32>,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, phoxal::Config)]
#[serde(rename_all = "snake_case")]
pub enum DeviceMode {
    #[default]
    Auto,
    Name,
    Index,
}

#[derive(Clone, Debug, Deserialize, Serialize, phoxal::Config)]
#[serde(deny_unknown_fields)]
pub struct AxisConfig {
    pub axis: Axis,
    #[serde(default = "deadzone")]
    pub deadzone: f64,
    #[serde(default)]
    pub invert: bool,
    pub scale: f64,
}

// Defaults are populated by Config rather than sharing dimensional scales.
impl Default for AxisConfig {
    fn default() -> Self {
        Self {
            axis: Axis::LeftY,
            deadzone: deadzone(),
            invert: false,
            scale: 0.5,
        }
    }
}
fn deadzone() -> f64 {
    0.12
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, phoxal::Config, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    LeftX,
    LeftY,
    RightX,
    RightY,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, phoxal::Config)]
#[serde(rename_all = "snake_case")]
pub enum Button {
    #[default]
    LeftBumper,
    RightBumper,
    South,
    East,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device: DeviceSelection::default(),
            linear: AxisConfig::default(),
            angular: AxisConfig {
                axis: Axis::RightX,
                scale: 1.5,
                invert: true,
                ..AxisConfig::default()
            },
            deadman: Button::LeftBumper,
        }
    }
}

impl Config {
    pub fn validate(&self) -> phoxal::Result<()> {
        for (name, axis) in [("linear", &self.linear), ("angular", &self.angular)] {
            if !axis.deadzone.is_finite() || !(0.0..1.0).contains(&axis.deadzone) {
                return Err(phoxal::anyhow!("{name}.deadzone must be finite in [0,1)"));
            }
            if !axis.scale.is_finite() || axis.scale <= 0.0 {
                return Err(phoxal::anyhow!("{name}.scale must be finite and positive"));
            }
        }
        match self.device.mode {
            DeviceMode::Auto if self.device.name.is_none() && self.device.index.is_none() => {}
            DeviceMode::Name
                if self.device.index.is_none()
                    && self.device.name.as_ref().is_some_and(|name| {
                        !name.trim().is_empty()
                            && name.trim() == name
                            && name.len() <= 256
                            && !name.contains('\0')
                    }) => {}
            DeviceMode::Index
                if self.device.name.is_none()
                    && self.device.index.is_some_and(|index| index <= 65535) => {}
            _ => {
                return Err(phoxal::anyhow!(
                    "device requires auto with no selector, name with one trimmed nonempty name, or index in 0..=65535"
                ));
            }
        }
        Ok(())
    }
}

pub fn mapped_axis(raw: f64, config: &AxisConfig) -> Option<f64> {
    if !raw.is_finite() || !(-1.0..=1.0).contains(&raw) {
        return None;
    }
    let value = raw.signum() * ((raw.abs() - config.deadzone).max(0.0) / (1.0 - config.deadzone));
    Some(value * config.scale * if config.invert { -1.0 } else { 1.0 })
}

fn angular_default() -> AxisConfig {
    Config::default().angular
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_shape_and_axis_semantics() {
        let config: Config = serde_json::from_str("{}").unwrap();
        config.validate().unwrap();
        assert_eq!(config.angular.axis, Axis::RightX);
        assert_eq!(mapped_axis(0.12, &config.linear), Some(0.0));
        assert_eq!(mapped_axis(-0.12, &config.linear), Some(-0.0));
        assert_eq!(mapped_axis(1.0, &config.linear), Some(0.5));
        assert_eq!(mapped_axis(-1.0, &config.linear), Some(-0.5));
        assert_eq!(mapped_axis(1.0, &config.angular), Some(-1.5));
        assert!((mapped_axis(0.56, &config.linear).unwrap() - 0.25).abs() < 1e-12);
        for raw in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.01, -1.01] {
            assert!(mapped_axis(raw, &config.linear).is_none());
        }
    }
    #[test]
    fn invalid_configuration_is_rejected() {
        for deadzone in [f64::NAN, f64::INFINITY, -0.1, 1.0] {
            let mut config = Config::default();
            config.linear.deadzone = deadzone;
            assert!(config.validate().is_err());
        }
        for scale in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
            let mut config = Config::default();
            config.angular.scale = scale;
            assert!(config.validate().is_err());
        }
        for name in ["", " ", " padded", "nul\0"] {
            let config = Config {
                device: DeviceSelection {
                    mode: DeviceMode::Name,
                    name: Some(name.into()),
                    index: None,
                },
                ..Config::default()
            };
            assert!(config.validate().is_err());
        }
        let mut config = Config::default();
        config.device.index = Some(0);
        assert!(config.validate().is_err());
        config.device.mode = DeviceMode::Index;
        config.validate().unwrap();
        config.device.index = Some(65536);
        assert!(config.validate().is_err());
    }
}
