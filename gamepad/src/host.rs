//! A bounded, nonblocking OS poll at the runtime invocation boundary.
use crate::config::{Axis, Button, Config};
use gilrs::{Axis as GilrsAxis, Button as GilrsButton, EventType, Gilrs, GilrsBuilder};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_EVENTS: usize = 256;
const MAX_DEVICES: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub index: u32,
    pub incarnation: u64,
}
#[derive(Clone, Debug)]
pub struct Device {
    pub key: Key,
    pub name: String,
    pub linear: f64,
    pub angular: f64,
    pub held: bool,
    pub interrupted: bool,
    pub supported: bool,
}
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub devices: Vec<Device>,
    pub fault: Option<String>,
}

pub trait Input: Send {
    fn poll(&mut self, config: &Config) -> Frame;
}

pub struct Host {
    backend: Box<Gilrs>,
    incarnations: BTreeMap<u32, u64>,
    serial: u64,
}

impl Host {
    pub fn new() -> phoxal::Result<Self> {
        let backend = GilrsBuilder::new()
            .with_default_filters(false)
            .build()
            .map_err(|error| phoxal::anyhow!("gamepad OS backend could not initialize: {error}"))?;
        Ok(Self {
            backend: Box::new(backend),
            incarnations: BTreeMap::new(),
            serial: 0,
        })
    }

    pub fn poll(&mut self, config: &Config) -> Frame {
        let backend = &mut self.backend;
        let mut interrupted = BTreeSet::new();
        for _ in 0..MAX_EVENTS {
            let Some(event) = backend.next_event() else {
                break;
            };
            let index = usize::from(event.id) as u32;
            match event.event {
                EventType::Connected | EventType::Disconnected => {
                    self.serial = match self.serial.checked_add(1) {
                        Some(serial) => serial,
                        None => return failed("controller incarnation counter exhausted"),
                    };
                    self.incarnations.insert(index, self.serial);
                    interrupted.insert(index);
                }
                EventType::ButtonReleased(button, _) if button == gilrs_button(config.deadman) => {
                    interrupted.insert(index);
                }
                _ => {}
            }
        }
        // One bounded lookahead detects a backlog; cached state is not trusted until drained.
        if backend.next_event().is_some() {
            return failed(
                "gamepad event backlog exceeded 256 events; release deadman after the queue drains",
            );
        }
        let mut devices = Vec::new();
        for (id, device) in backend.gamepads().take(MAX_DEVICES + 1) {
            if devices.len() == MAX_DEVICES {
                return failed("more than 16 connected controllers");
            }
            let index = usize::from(id) as u32;
            devices.push(Device {
                key: Key {
                    index,
                    incarnation: *self.incarnations.get(&index).unwrap_or(&0),
                },
                name: device.os_name().chars().take(256).collect(),
                linear: f64::from(device.value(gilrs_axis(config.linear.axis))),
                angular: f64::from(device.value(gilrs_axis(config.angular.axis))),
                held: device.is_pressed(gilrs_button(config.deadman)),
                interrupted: interrupted.contains(&index),
                supported: device.axis_code(gilrs_axis(config.linear.axis)).is_some()
                    && device.axis_code(gilrs_axis(config.angular.axis)).is_some()
                    && device.button_code(gilrs_button(config.deadman)).is_some(),
            });
        }
        // Disconnected IDs need no retained state; reconnect gets a fresh serial.
        self.incarnations
            .retain(|id, _| devices.iter().any(|device| device.key.index == *id));
        Frame {
            devices,
            fault: None,
        }
    }
}
fn failed(message: &str) -> Frame {
    Frame {
        fault: Some(message.into()),
        ..Default::default()
    }
}
impl Input for Host {
    fn poll(&mut self, config: &Config) -> Frame {
        Host::poll(self, config)
    }
}

fn gilrs_axis(axis: Axis) -> GilrsAxis {
    match axis {
        Axis::LeftX => GilrsAxis::LeftStickX,
        Axis::LeftY => GilrsAxis::LeftStickY,
        Axis::RightX => GilrsAxis::RightStickX,
        Axis::RightY => GilrsAxis::RightStickY,
    }
}
fn gilrs_button(button: Button) -> GilrsButton {
    match button {
        Button::LeftBumper => GilrsButton::LeftTrigger,
        Button::RightBumper => GilrsButton::RightTrigger,
        Button::South => GilrsButton::South,
        Button::East => GilrsButton::East,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a native OS gamepad backend; does not qualify physical driving"]
    fn native_backend_snapshot() {
        let mut host = Host::new().expect("native backend initializes");
        let mut frame = host.poll(&Config::default());
        // Native device enumeration can arrive asynchronously after backend
        // construction. This bounded host qualification samples that startup.
        for _ in 0..50 {
            if !frame.devices.is_empty() || frame.fault.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
            frame = host.poll(&Config::default());
        }
        println!("native OS snapshot after bounded enumeration: {frame:?}");
        assert!(
            frame.fault.is_none(),
            "native bounded polling fault: {:?}",
            frame.fault
        );
    }
}
