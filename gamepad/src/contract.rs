//! Motion owns these messages; this is a private structural expectation checked at composition.
use phoxal::contracts::robotics::MotionSetpoint;
use phoxal::contracts::{Empty, Latest, RequestReply};

#[phoxal::messages(package = "phoxal.motion.v1")]
pub mod motion {
    pub enum ControlMode {
        Unspecified = 0,
        Disarmed = 1,
        Manual = 2,
        Autonomous = 3,
    }
    pub struct ArmRequest {
        #[phoxal(tag = 1)]
        pub mode: ControlMode,
    }
    pub enum EmergencyRefusalReason {
        Unspecified = 0,
        InvalidRequest = 1,
        ProtectiveState = 2,
        NotLatched = 3,
    }
    pub struct EmergencyRefused {
        #[phoxal(tag = 1)]
        pub reason: EmergencyRefusalReason,
    }
    pub enum ApplyEmergencyResponse {
        #[phoxal(tag = 1)]
        Accepted,
        #[phoxal(tag = 2)]
        Refused(EmergencyRefused),
    }
}

#[phoxal::messages(package = "phoxal.gamepad.v1")]
mod v1 {
    use super::{Empty, Latest, MotionSetpoint, RequestReply, motion};
    pub enum Phase {
        NoController = 0,
        ReleaseRequired = 1,
        Ready = 2,
        Priming = 3,
        Arming = 4,
        Manual = 5,
        Stopping = 6,
        Fault = 7,
    }
    pub struct Status {
        #[phoxal(tag = 1)]
        pub phase: Phase,
        #[phoxal(tag = 2)]
        pub device_index: Option<u32>,
        #[phoxal(tag = 3)]
        pub diagnostic: Option<String>,
    }
    #[phoxal::endpoints]
    pub struct GamepadApi {
        #[phoxal::output(projection = state, lease_ms = 100, max_bytes = 256)]
        intent: Latest<MotionSetpoint>,
        #[phoxal::output(projection = state, bootstrap, max_bytes = 1024)]
        status: Latest<Status>,
        #[phoxal::call(contract = "phoxal.motion.v1.Arm", max_items = 2, max_bytes = 1024)]
        arm_motion: RequestReply<motion::ArmRequest, motion::ApplyEmergencyResponse>,
        #[phoxal::call(contract = "phoxal.motion.v1.Disarm", max_items = 2, max_bytes = 1024)]
        disarm_motion: RequestReply<Empty, motion::ApplyEmergencyResponse>,
    }
}
pub use v1::*;
