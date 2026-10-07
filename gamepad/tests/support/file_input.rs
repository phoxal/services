//! Bounded file orchestration confined to the standalone test fixture.
use crate::{
    config::Config,
    host::{Device, Frame, Input, Key},
};
use std::path::PathBuf;
pub struct FileInput {
    pub path: PathBuf,
    pub previous: String,
    pub incarnation: u64,
}
impl Input for FileInput {
    fn poll(&mut self, _: &Config) -> Frame {
        use std::io::Read as _;
        let read = || -> std::io::Result<String> {
            let mut bytes = Vec::new();
            std::fs::File::open(&self.path)?
                .take(33)
                .read_to_end(&mut bytes)?;
            if bytes.len() > 32 {
                return Err(std::io::Error::other("fixture input exceeds bound"));
            }
            String::from_utf8(bytes).map_err(std::io::Error::other)
        };
        let value = match read() {
            Ok(value) => value,
            Err(error) => {
                return Frame {
                    fault: Some(error.to_string()),
                    ..Frame::default()
                };
            }
        };
        let value = value.trim();
        if value == "disconnected" {
            self.previous = value.into();
            return Frame::default();
        }
        if !matches!(
            value,
            "released"
                | "held"
                | "drive"
                | "release_repress"
                | "reconnected_held"
                | "reconnected_released"
        ) {
            return Frame {
                fault: Some("unknown fixture input".into()),
                ..Frame::default()
            };
        }
        let reconnected = value != self.previous
            && (value.starts_with("reconnected_") || self.previous == "disconnected");
        if reconnected {
            self.incarnation += 1;
        }
        let interrupted = reconnected || (value == "release_repress" && value != self.previous);
        self.previous = value.into();
        Frame {
            devices: vec![Device {
                key: Key {
                    index: 0,
                    incarnation: self.incarnation,
                },
                name: "fixture controller".into(),
                linear: if matches!(value, "drive" | "release_repress") {
                    1.0
                } else {
                    0.0
                },
                angular: 0.0,
                held: !matches!(value, "released" | "reconnected_released"),
                interrupted,
                supported: true,
            }],
            fault: None,
        }
    }
}
