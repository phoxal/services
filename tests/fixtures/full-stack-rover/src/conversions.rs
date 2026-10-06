//! Robot-owned conversions between published payloads and the private
//! typed expectations of the selected services.

use crate::api::motion;
use crate::api::motion as motion_expectation;
use crate::api::motion::ControlMode as PublishedControlMode;
use crate::api::safety;
use crate::api::safety as safety_published;
use crate::api::safety::ControlMode as ExpectedControlMode;
use crate::api::world;

impl From<world::WorldBelief> for safety::WorldBelief {
    fn from(source: world::WorldBelief) -> safety::WorldBelief {
        safety::WorldBelief {
            frame_id: source.frame_id,
            x_m: source.x_m,
            y_m: source.y_m,
            yaw_rad: source.yaw_rad,
            confidence: source.confidence,
            revision: source.revision,
            available: source.available,
            oldest_capture_time_nanos: source.oldest_capture_time_nanos,
        }
    }
}

impl From<world::WorldRevision> for safety::WorldRevision {
    fn from(source: world::WorldRevision) -> safety::WorldRevision {
        safety::WorldRevision {
            revision: source.revision,
            available: source.available,
            oldest_capture_time_nanos: source.oldest_capture_time_nanos,
        }
    }
}

impl From<motion::MotionStatus> for safety::MotionStatus {
    fn from(source: motion::MotionStatus) -> safety::MotionStatus {
        safety::MotionStatus {
            mode: match source.mode {
                PublishedControlMode::Unspecified => ExpectedControlMode::Unspecified,
                PublishedControlMode::Disarmed => ExpectedControlMode::Disarmed,
                PublishedControlMode::Manual => ExpectedControlMode::Manual,
                PublishedControlMode::Autonomous => ExpectedControlMode::Autonomous,
            },
            emergency_latched: source.emergency_latched,
            selected_owner_id: source.selected_owner_id,
            protective_state_clear: source.protective_state_clear,
            stopped: source.stopped,
        }
    }
}

impl From<safety::MotionConstraints> for motion::MotionConstraints {
    fn from(source: safety::MotionConstraints) -> motion::MotionConstraints {
        motion::MotionConstraints {
            sequence: source.sequence,
            permission: match source.permission {
                safety_published::Permission::Unspecified => {
                    motion_expectation::Permission::Unspecified
                }
                safety_published::Permission::Clear => motion_expectation::Permission::Clear,
                safety_published::Permission::Limited => motion_expectation::Permission::Limited,
                safety_published::Permission::Stopped => motion_expectation::Permission::Stopped,
            },
            constraints: source
                .constraints
                .into_iter()
                .map(|constraint| motion_expectation::Constraint {
                    reason: match constraint.reason {
                        safety_published::ConstraintReason::Unspecified => {
                            motion_expectation::ConstraintReason::Unspecified
                        }
                        safety_published::ConstraintReason::WorldUnavailable => {
                            motion_expectation::ConstraintReason::WorldUnavailable
                        }
                        safety_published::ConstraintReason::LocalizationUncertain => {
                            motion_expectation::ConstraintReason::LocalizationUncertain
                        }
                        safety_published::ConstraintReason::MapUnavailable => {
                            motion_expectation::ConstraintReason::MapUnavailable
                        }
                        safety_published::ConstraintReason::MapBlocked => {
                            motion_expectation::ConstraintReason::MapBlocked
                        }
                        safety_published::ConstraintReason::RangeUnavailable => {
                            motion_expectation::ConstraintReason::RangeUnavailable
                        }
                        safety_published::ConstraintReason::RangeFault => {
                            motion_expectation::ConstraintReason::RangeFault
                        }
                        safety_published::ConstraintReason::ObstacleProximity => {
                            motion_expectation::ConstraintReason::ObstacleProximity
                        }
                        safety_published::ConstraintReason::MotionUnavailable => {
                            motion_expectation::ConstraintReason::MotionUnavailable
                        }
                        safety_published::ConstraintReason::MotionFault => {
                            motion_expectation::ConstraintReason::MotionFault
                        }
                    },
                    max_linear_speed_mps: constraint.max_linear_speed_mps,
                    max_angular_speed_radps: constraint.max_angular_speed_radps,
                    observed_value: constraint.observed_value,
                })
                .collect(),
            valid_from_nanos: source.valid_from_nanos,
            expires_at_nanos: source.expires_at_nanos,
            oldest_capture_time_nanos: source.oldest_capture_time_nanos,
        }
    }
}

impl From<world::WorldRevision> for crate::api::navigation::MapState {
    fn from(source: world::WorldRevision) -> Self {
        Self {
            revision: source.revision,
            available: source.available,
            oldest_capture_time_nanos: source.oldest_capture_time_nanos,
        }
    }
}
