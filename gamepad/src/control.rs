//! Pure deadman/authority transitions. Network effects are staged by the runtime owner.
use crate::{
    config::{Config, DeviceMode, mapped_axis},
    contract::Phase,
    host::{Device, Frame, Key},
};
use phoxal::contracts::robotics::MotionSetpoint;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Accepted,
    Refused,
    NotSent,
    Unknown,
}
#[derive(Default, Debug)]
pub struct Effects {
    pub arm: bool,
    pub disarm: bool,
}

pub struct Control {
    pub phase: Phase,
    pub selected: Option<Key>,
    pub intent: Option<MotionSetpoint>,
    pub diagnostic: Option<String>,
    release_seen: bool,
    primed_at: Option<u64>,
    arm_pending: bool,
    disarm_pending: bool,
    authority_possible: bool,
    cleanup_again: bool,
    uncertain: bool,
    uncertain_cleanup_attempted: bool,
}
impl Default for Control {
    fn default() -> Self {
        Self {
            phase: Phase::NoController,
            selected: None,
            intent: None,
            diagnostic: None,
            release_seen: false,
            primed_at: None,
            arm_pending: false,
            disarm_pending: false,
            authority_possible: false,
            cleanup_again: false,
            uncertain: false,
            uncertain_cleanup_attempted: false,
        }
    }
}
impl Control {
    pub fn update(
        &mut self,
        config: &Config,
        frame: &Frame,
        invocation: u64,
        arm_reply: Option<Outcome>,
        disarm_reply: Option<Outcome>,
    ) -> Effects {
        let mut effects = Effects::default();
        if let Some(reply) = disarm_reply {
            self.disarm_pending = false;
            match reply {
                Outcome::Accepted => self.authority_possible = self.arm_pending,
                _ => {
                    self.uncertain = true;
                    self.diagnostic =
                        Some("disarm was not acknowledged; stop/reset the execution".into());
                }
            }
        }
        if let Some(reply) = arm_reply {
            self.arm_pending = false;
            match reply {
                Outcome::Accepted if self.phase == Phase::Arming => self.phase = Phase::Manual,
                Outcome::Accepted => {
                    self.authority_possible = true;
                    self.cleanup_again = true;
                }
                Outcome::Refused | Outcome::NotSent => {
                    self.intent = None;
                    self.release_seen = false;
                    self.phase = Phase::ReleaseRequired;
                    self.authority_possible = self.disarm_pending;
                    self.diagnostic = Some(
                        "arm refused or not sent; center sticks, release, then press again".into(),
                    );
                }
                Outcome::Unknown => {
                    self.intent = None;
                    self.uncertain = true;
                    self.authority_possible = true;
                    self.diagnostic = Some(
                        "arm outcome unknown; stop/reset the execution before reengaging".into(),
                    );
                }
            }
        }
        if self.disarm_pending && self.phase != Phase::Stopping && !self.uncertain {
            self.intent = None;
            self.phase = Phase::Stopping;
        }
        let selected_device = self
            .selected
            .and_then(|key| frame.devices.iter().find(|device| device.key == key));
        let interrupted = frame.fault.is_some()
            || selected_device
                .is_none_or(|device| !device.supported || !device.held || device.interrupted);
        if self.uncertain
            || ((self.phase == Phase::Manual
                || self.phase == Phase::Arming
                || self.phase == Phase::Priming)
                && interrupted)
        {
            self.intent = None;
            self.release_seen = false;
            self.primed_at = None;
            self.phase = if self.uncertain {
                Phase::Fault
            } else {
                Phase::Stopping
            };
            if self.authority_possible
                && !self.disarm_pending
                && (!self.uncertain || !self.uncertain_cleanup_attempted)
            {
                self.disarm_pending = true;
                self.cleanup_again = false;
                effects.disarm = true;
                self.uncertain_cleanup_attempted |= self.uncertain;
            }
        }
        if self.cleanup_again && !self.disarm_pending {
            self.disarm_pending = true;
            self.cleanup_again = false;
            self.phase = Phase::Stopping;
            self.intent = None;
            effects.disarm = true;
        }
        if self.uncertain {
            self.phase = Phase::Fault;
            return effects;
        }
        if self.phase == Phase::Stopping {
            if self.arm_pending || self.disarm_pending {
                return effects;
            }
            self.authority_possible = false;
            self.phase = Phase::ReleaseRequired;
            self.selected = None;
        }
        if let Some(fault) = &frame.fault {
            self.intent = None;
            self.release_seen = false;
            self.selected = None;
            self.phase = Phase::Fault;
            self.diagnostic = Some(fault.chars().take(512).collect());
            return effects;
        }
        if self.selected.is_none() || selected_device.is_none() {
            self.selected = None;
            let matches: Vec<_> = frame
                .devices
                .iter()
                .filter(|device| match config.device.mode {
                    DeviceMode::Auto => device.supported,
                    DeviceMode::Name => config.device.name.as_ref() == Some(&device.name),
                    DeviceMode::Index => Some(device.key.index) == config.device.index,
                })
                .collect();
            self.intent = None;
            self.release_seen = false;
            if matches.len() != 1 {
                self.phase = Phase::NoController;
                self.diagnostic = (matches.len() > 1).then(|| {
                    "controller selection is ambiguous; select an exact name or host index".into()
                });
                return effects;
            }
            self.selected = Some(matches[0].key);
            self.phase = Phase::ReleaseRequired;
        }
        let Some(device) = frame
            .devices
            .iter()
            .find(|device| Some(device.key) == self.selected)
        else {
            return effects;
        };
        let Some(value) = mapped(device, config) else {
            self.intent = None;
            self.release_seen = false;
            self.phase = if self.authority_possible {
                Phase::Stopping
            } else {
                Phase::Fault
            };
            self.diagnostic =
                Some("controller mapping is unavailable or its axes are invalid".into());
            if self.authority_possible && !self.disarm_pending {
                self.disarm_pending = true;
                effects.disarm = true;
            }
            return effects;
        };
        if device.interrupted {
            self.intent = None;
            self.release_seen = false;
            self.phase = Phase::ReleaseRequired;
            return effects;
        }
        match self.phase {
            Phase::NoController | Phase::ReleaseRequired | Phase::Ready | Phase::Fault => {
                self.intent = None;
                if !device.held {
                    self.release_seen = true;
                    self.phase = Phase::Ready;
                    self.diagnostic = None;
                } else if self.release_seen
                    && value.linear_x_mps == 0.0
                    && value.angular_z_radps == 0.0
                {
                    self.phase = Phase::Priming;
                    self.primed_at = Some(invocation);
                    self.intent = Some(neutral());
                    self.diagnostic = None;
                } else {
                    self.release_seen = false;
                    self.phase = Phase::ReleaseRequired;
                    self.diagnostic =
                        Some("center sticks, release deadman, then press to engage".into());
                }
            }
            Phase::Priming => {
                self.intent = Some(neutral());
                if self.primed_at.is_some_and(|first| invocation > first) {
                    self.phase = Phase::Arming;
                    self.arm_pending = true;
                    self.authority_possible = true;
                    effects.arm = true;
                }
            }
            Phase::Arming => self.intent = Some(neutral()),
            Phase::Manual => self.intent = Some(value),
            Phase::Stopping => self.intent = None,
        }
        effects
    }
}
fn neutral() -> MotionSetpoint {
    MotionSetpoint {
        linear_x_mps: 0.0,
        angular_z_radps: 0.0,
    }
}
fn mapped(device: &Device, config: &Config) -> Option<MotionSetpoint> {
    device.supported.then_some(())?;
    Some(MotionSetpoint {
        linear_x_mps: mapped_axis(device.linear, &config.linear)?,
        angular_z_radps: mapped_axis(device.angular, &config.angular)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    pub fn frame(held: bool, linear: f64) -> Frame {
        Frame {
            devices: vec![Device {
                key: Key {
                    index: 0,
                    incarnation: 0,
                },
                name: "controller".into(),
                linear,
                angular: 0.0,
                held,
                interrupted: false,
                supported: true,
            }],
            fault: None,
        }
    }
    fn armed() -> Control {
        let mut control = Control::default();
        let config = Config::default();
        control.update(&config, &frame(false, 0.0), 0, None, None);
        control.update(&config, &frame(true, 0.0), 1, None, None);
        assert!(
            control
                .update(&config, &frame(true, 0.0), 2, None, None)
                .arm
        );
        control.update(&config, &frame(true, 1.0), 3, Some(Outcome::Accepted), None);
        assert_eq!(control.phase, Phase::Manual);
        control
    }
    #[test]
    fn startup_held_and_non_neutral_press_cannot_arm() {
        let mut control = Control::default();
        let config = Config::default();
        for index in 0..3 {
            assert!(
                !control
                    .update(&config, &frame(true, 0.0), index, None, None)
                    .arm
            );
        }
        control.update(&config, &frame(false, 0.0), 3, None, None);
        control.update(&config, &frame(true, 1.0), 4, None, None);
        assert_eq!(control.phase, Phase::ReleaseRequired);
        assert!(control.intent.is_none());
        control.update(&config, &frame(false, 0.0), 5, None, None);
        control.update(&config, &frame(true, 0.0), 6, None, None);
        assert_eq!(control.intent, Some(neutral()));
        assert!(
            !control
                .update(&config, &frame(true, 0.0), 6, None, None)
                .arm
        );
        assert!(
            control
                .update(&config, &frame(true, 0.0), 7, None, None)
                .arm
        );
    }
    #[test]
    fn release_disconnect_invalid_axes_and_backend_loss_withdraw_once() {
        let mut invalid = frame(true, f64::NAN);
        invalid.devices[0].angular = f64::INFINITY;
        for lost in [
            frame(false, 1.0),
            Frame::default(),
            invalid,
            Frame {
                fault: Some("backend lost".into()),
                ..Default::default()
            },
        ] {
            let mut control = armed();
            let config = Config::default();
            assert_eq!(control.intent.unwrap().linear_x_mps, 0.5);
            assert!(control.update(&config, &lost, 4, None, None).disarm);
            assert!(control.intent.is_none());
            assert!(!control.update(&config, &lost, 5, None, None).disarm);
            assert!(
                !control
                    .update(&config, &lost, 6, None, Some(Outcome::Accepted))
                    .disarm
            );
            assert!(control.intent.is_none());
            control.update(&config, &frame(true, 0.0), 7, None, None);
            assert_ne!(control.phase, Phase::Manual);
        }
    }
    #[test]
    fn late_arm_success_requires_another_cleanup_after_prior_disarm() {
        let mut control = Control::default();
        let config = Config::default();
        control.update(&config, &frame(false, 0.0), 0, None, None);
        control.update(&config, &frame(true, 0.0), 1, None, None);
        control.update(&config, &frame(true, 0.0), 2, None, None);
        assert!(
            control
                .update(&config, &frame(false, 0.0), 3, None, None)
                .disarm
        );
        control.update(
            &config,
            &frame(false, 0.0),
            4,
            None,
            Some(Outcome::Accepted),
        );
        assert!(
            control
                .update(&config, &frame(true, 0.0), 5, Some(Outcome::Accepted), None)
                .disarm
        );
        assert!(control.intent.is_none());
        control.update(&config, &frame(true, 0.0), 6, None, Some(Outcome::Accepted));
        assert_eq!(control.phase, Phase::ReleaseRequired);
    }
    #[test]
    fn unknown_outcomes_latch_without_repeated_disarm() {
        let mut control = armed();
        let config = Config::default();
        assert!(
            control
                .update(&config, &frame(true, 0.0), 4, Some(Outcome::Unknown), None)
                .disarm
        );
        assert!(
            !control
                .update(&config, &frame(true, 0.0), 5, None, Some(Outcome::Unknown))
                .disarm
        );
        for index in 6..10 {
            let effects = control.update(&config, &frame(false, 0.0), index, None, None);
            assert!(!effects.arm && !effects.disarm);
            assert!(control.intent.is_none());
            assert_eq!(control.phase, Phase::Fault);
        }
    }
    #[test]
    fn refused_arm_and_idle_require_fresh_engagement_without_disarming() {
        let config = Config::default();
        let mut control = Control::default();
        for index in 0..4 {
            let effects = control.update(&config, &Frame::default(), index, None, None);
            assert!(!effects.arm && !effects.disarm);
        }
        control.update(&config, &frame(false, 0.0), 4, None, None);
        control.update(&config, &frame(true, 0.0), 5, None, None);
        control.update(&config, &frame(true, 0.0), 6, None, None);
        let effects = control.update(&config, &frame(true, 0.0), 7, Some(Outcome::Refused), None);
        assert!(!effects.arm && !effects.disarm);
        assert!(control.intent.is_none());
        assert!(
            !control
                .update(&config, &frame(true, 0.0), 8, None, None)
                .arm
        );
        control.update(&config, &frame(false, 0.0), 9, None, None);
        control.update(&config, &frame(true, 0.0), 10, None, None);
        assert!(
            control
                .update(&config, &frame(true, 0.0), 11, None, None)
                .arm
        );
    }

    #[test]
    fn ambiguous_selection_and_reconnect_cannot_engage() {
        let mut control = Control::default();
        let config = Config::default();
        let mut duplicate = frame(false, 0.0);
        let mut second = duplicate.devices[0].clone();
        second.key.index = 1;
        duplicate.devices.push(second);
        control.update(&config, &duplicate, 0, None, None);
        assert_eq!(control.phase, Phase::NoController);
        let mut control = armed();
        let mut replacement = frame(true, 0.0);
        replacement.devices[0].key.incarnation = 1;
        assert!(control.update(&config, &replacement, 4, None, None).disarm);
        control.update(&config, &replacement, 5, None, Some(Outcome::Accepted));
        assert_eq!(control.phase, Phase::ReleaseRequired);
    }
}
