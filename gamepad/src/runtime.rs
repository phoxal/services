use crate::{
    config::Config,
    contract::{
        GamepadApi, Status,
        motion::{ApplyEmergencyResponse, ArmRequest, ControlMode},
    },
    control::{Control, Outcome},
    host::{Host, Input},
};
use phoxal::{
    contracts::{Empty, robotics::MotionSetpoint},
    runtime::{CallCompletion, CallTicket, Context, ExecutionTime, input::RequestError},
};

const CALL_TIMEOUT_NS: u64 = 500_000_000;
struct Pending {
    ticket: CallTicket<ApplyEmergencyResponse>,
    at: ExecutionTime,
}
pub struct Gamepad {
    config: Config,
    host: Box<dyn Input>,
    control: Control,
    arm: Option<Pending>,
    disarm: Option<Pending>,
}
#[phoxal::runtime(contract = GamepadApi, period_ms = 20, init_timeout_ms = 5000)]
impl Gamepad {
    #[init]
    fn new(config: Config) -> phoxal::Result<Self> {
        Self::with_input(config, || Ok(Box::new(Host::new()?)))
    }

    #[complete(arm_motion)]
    fn arm_completed(
        &mut self,
        _ctx: &mut Context<'_, Self>,
        _reply: CallCompletion<ApplyEmergencyResponse>,
    ) -> phoxal::Result<()> {
        Ok(())
    }
    #[complete(disarm_motion)]
    fn disarm_completed(
        &mut self,
        _ctx: &mut Context<'_, Self>,
        _reply: CallCompletion<ApplyEmergencyResponse>,
    ) -> phoxal::Result<()> {
        Ok(())
    }
    #[step]
    fn sample(&mut self, ctx: &mut Context<'_, Self>) -> phoxal::Result<()> {
        // Fresh input is sampled before handling a late arm response.
        let frame = self.host.poll(&self.config);
        let arm_reply = poll(ctx, &mut self.arm);
        let disarm_reply = poll(ctx, &mut self.disarm);
        let effects = self.control.update(
            &self.config,
            &frame,
            ctx.invocation_index(),
            arm_reply,
            disarm_reply,
        );
        if effects.arm {
            self.arm = Some(Pending {
                ticket: ctx.send(GamepadApi::arm_motion(ArmRequest {
                    mode: ControlMode::Manual,
                }))?,
                at: ctx.now(),
            });
        }
        if effects.disarm {
            self.disarm = Some(Pending {
                ticket: ctx.send(GamepadApi::disarm_motion(Empty {}))?,
                at: ctx.now(),
            });
        }
        Ok(())
    }
    #[publish(intent)]
    fn intent(&self) -> Option<MotionSetpoint> {
        self.control.intent
    }
    #[publish(status)]
    fn status(&self) -> Status {
        Status {
            phase: self.control.phase,
            device_index: self.control.selected.map(|key| key.index),
            diagnostic: self.control.diagnostic.clone(),
        }
    }
}
impl Gamepad {
    fn with_input(
        config: Config,
        input: impl FnOnce() -> phoxal::Result<Box<dyn Input>>,
    ) -> phoxal::Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            host: input()?,
            control: Control::default(),
            arm: None,
            disarm: None,
        })
    }
}

fn poll(ctx: &mut Context<'_, Gamepad>, pending: &mut Option<Pending>) -> Option<Outcome> {
    let call = pending.as_ref()?;
    if let Some(reply) = ctx.take_completion(&call.ticket) {
        let result = match reply.into_result() {
            Ok(ApplyEmergencyResponse::Accepted) => Outcome::Accepted,
            Ok(ApplyEmergencyResponse::Refused(_)) => Outcome::Refused,
            Err(RequestError::NotSent(_) | RequestError::RejectedBeforeAdmission(_)) => {
                Outcome::NotSent
            }
            Err(_) => Outcome::Unknown,
        };
        *pending = None;
        return Some(result);
    }
    if ctx.now().as_nanos().saturating_sub(call.at.as_nanos()) >= CALL_TIMEOUT_NS {
        ctx.retire_completion(&call.ticket);
        *pending = None;
        return Some(Outcome::Unknown);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::fixture::LocalHarness;
    use super::*;
    use crate::contract::Phase;
    use crate::host::{Device, Frame, Key};
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };
    fn input(held: bool, linear: f64) -> Frame {
        Frame {
            devices: vec![Device {
                key: Key {
                    index: 0,
                    incarnation: 0,
                },
                name: "test-only controller".into(),
                held,
                linear,
                angular: 0.0,
                supported: true,
                interrupted: false,
            }],
            fault: None,
        }
    }
    fn harness(frame: &Arc<Mutex<Frame>>) -> LocalHarness {
        fixture::harness(frame.clone()).unwrap()
    }
    #[test]
    fn accepted_runtime_primes_handles_reply_and_withdraws_on_fresh_release() {
        let frame = Arc::new(Mutex::new(input(false, 0.0)));
        let mut runtime = harness(&frame);
        runtime.advance_to(Duration::ZERO).unwrap();
        assert_eq!(runtime.status().unwrap().phase, Phase::Ready);
        *frame.lock().unwrap() = input(true, 0.0);
        runtime.advance_to(Duration::from_millis(20)).unwrap();
        assert!(
            runtime
                .take_request::<ArmRequest, ApplyEmergencyResponse>("arm_motion")
                .unwrap()
                .is_none()
        );
        runtime.advance_to(Duration::from_millis(40)).unwrap();
        let request = runtime
            .take_request::<ArmRequest, ApplyEmergencyResponse>("arm_motion")
            .unwrap()
            .unwrap();
        assert_eq!(request.request().mode, ControlMode::Manual);
        runtime
            .complete_request(&request, Ok(ApplyEmergencyResponse::Accepted))
            .unwrap();
        *frame.lock().unwrap() = input(true, 1.0);
        runtime.advance_to(Duration::from_millis(60)).unwrap();
        assert_eq!(runtime.status().unwrap().phase, Phase::Manual);
        assert_eq!(runtime.intent().unwrap().linear_x_mps, 0.5);
        // Paused OS changes become visible only at the next accepted invocation.
        *frame.lock().unwrap() = input(false, 1.0);
        runtime.advance_to(Duration::from_millis(60)).unwrap();
        assert_eq!(runtime.status().unwrap().phase, Phase::Manual);
        runtime.advance_to(Duration::from_millis(80)).unwrap();
        assert_eq!(runtime.status().unwrap().phase, Phase::Stopping);
        assert!(runtime.intent().is_none());
        assert!(
            runtime
                .take_request::<Empty, ApplyEmergencyResponse>("disarm_motion")
                .unwrap()
                .is_some()
        );
    }
    #[test]
    fn reset_fences_old_arm_reply_and_requires_release() {
        let frame = Arc::new(Mutex::new(input(false, 0.0)));
        let mut runtime = harness(&frame);
        runtime.advance_to(Duration::ZERO).unwrap();
        *frame.lock().unwrap() = input(true, 0.0);
        runtime.advance_to(Duration::from_millis(40)).unwrap();
        let request = runtime
            .take_request::<ArmRequest, ApplyEmergencyResponse>("arm_motion")
            .unwrap()
            .unwrap();
        runtime.reset(Config::default()).unwrap();
        assert!(
            runtime
                .complete_request(&request, Ok(ApplyEmergencyResponse::Accepted))
                .is_err()
        );
        runtime.advance_to(Duration::from_millis(40)).unwrap();
        assert_eq!(runtime.status().unwrap().phase, Phase::ReleaseRequired);
        assert!(
            runtime
                .take_request::<ArmRequest, ApplyEmergencyResponse>("arm_motion")
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn logical_timeout_withdraws_and_latches_unknown_outcome() {
        let frame = Arc::new(Mutex::new(input(false, 0.0)));
        let mut runtime = harness(&frame);
        runtime.advance_to(Duration::ZERO).unwrap();
        *frame.lock().unwrap() = input(true, 0.0);
        runtime.advance_to(Duration::from_millis(40)).unwrap();
        let _request = runtime
            .take_request::<ArmRequest, ApplyEmergencyResponse>("arm_motion")
            .unwrap()
            .unwrap();
        runtime.advance_to(Duration::from_millis(540)).unwrap();
        assert_eq!(runtime.status().unwrap().phase, Phase::Fault);
        assert!(
            runtime
                .take_request::<Empty, ApplyEmergencyResponse>("disarm_motion")
                .unwrap()
                .is_some()
        );
        runtime.advance_to(Duration::from_millis(1080)).unwrap();
        assert_eq!(runtime.status().unwrap().phase, Phase::Fault);
        assert!(
            runtime
                .take_request::<Empty, ApplyEmergencyResponse>("disarm_motion")
                .unwrap()
                .is_none()
        );
    }

    fn pending_arm(
        frame: &Arc<Mutex<Frame>>,
    ) -> (
        LocalHarness,
        phoxal::runtime::HarnessRequest<ArmRequest, ApplyEmergencyResponse>,
    ) {
        let mut runtime = harness(frame);
        assert!(runtime.intent().is_none());
        runtime.advance_to(Duration::ZERO).unwrap();
        *frame.lock().unwrap() = input(true, 0.0);
        runtime.advance_to(Duration::from_millis(39)).unwrap();
        assert_eq!(runtime.status().unwrap().phase, Phase::Priming);
        runtime.advance_to(Duration::from_millis(40)).unwrap();
        let request = runtime
            .take_request::<ArmRequest, ApplyEmergencyResponse>("arm_motion")
            .unwrap()
            .unwrap();
        (runtime, request)
    }

    #[test]
    fn correlated_refusal_not_sent_and_unknown_results_withdraw_without_rearming() {
        for result in [
            Ok(ApplyEmergencyResponse::Refused(
                crate::contract::motion::EmergencyRefused {
                    reason: crate::contract::motion::EmergencyRefusalReason::ProtectiveState,
                },
            )),
            Err(RequestError::NotSent("fixture route unavailable".into())),
            Err(RequestError::RejectedBeforeAdmission(
                "fixture admission refusal".into(),
            )),
            Err(RequestError::Timeout),
        ] {
            let frame = Arc::new(Mutex::new(input(false, 0.0)));
            let (mut runtime, request) = pending_arm(&frame);
            let uncertain = matches!(result, Err(RequestError::Timeout));
            runtime.complete_request(&request, result).unwrap();
            runtime.advance_to(Duration::from_millis(60)).unwrap();
            assert!(runtime.intent().is_none());
            assert_eq!(
                runtime.status().unwrap().phase,
                if uncertain {
                    Phase::Fault
                } else {
                    Phase::ReleaseRequired
                }
            );
            let cleanup = runtime
                .take_request::<Empty, ApplyEmergencyResponse>("disarm_motion")
                .unwrap();
            assert_eq!(cleanup.is_some(), uncertain);
            runtime.advance_to(Duration::from_millis(80)).unwrap();
            assert!(
                runtime
                    .take_request::<ArmRequest, ApplyEmergencyResponse>("arm_motion")
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[test]
    fn release_before_late_arm_success_cleans_up_again_using_actual_tickets() {
        let frame = Arc::new(Mutex::new(input(false, 0.0)));
        let (mut runtime, arm) = pending_arm(&frame);
        *frame.lock().unwrap() = input(false, 0.0);
        runtime.advance_to(Duration::from_millis(60)).unwrap();
        let disarm = runtime
            .take_request::<Empty, ApplyEmergencyResponse>("disarm_motion")
            .unwrap()
            .unwrap();
        runtime
            .complete_request(&disarm, Ok(ApplyEmergencyResponse::Accepted))
            .unwrap();
        runtime.advance_to(Duration::from_millis(80)).unwrap();
        runtime
            .complete_request(&arm, Ok(ApplyEmergencyResponse::Accepted))
            .unwrap();
        *frame.lock().unwrap() = input(true, 0.0);
        runtime.advance_to(Duration::from_millis(100)).unwrap();
        assert!(runtime.intent().is_none());
        let cleanup = runtime
            .take_request::<Empty, ApplyEmergencyResponse>("disarm_motion")
            .unwrap()
            .unwrap();
        runtime
            .complete_request(&cleanup, Ok(ApplyEmergencyResponse::Accepted))
            .unwrap();
        runtime.advance_to(Duration::from_millis(120)).unwrap();
        assert_eq!(runtime.status().unwrap().phase, Phase::ReleaseRequired);
        assert!(
            runtime
                .take_request::<ArmRequest, ApplyEmergencyResponse>("arm_motion")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn sampled_disconnect_fault_and_reconnect_withdraw_through_real_owner() {
        for lost in [
            Frame::default(),
            Frame {
                fault: Some("event queue overflow".into()),
                ..Default::default()
            },
        ] {
            let frame = Arc::new(Mutex::new(input(false, 0.0)));
            let (mut runtime, arm) = pending_arm(&frame);
            runtime
                .complete_request(&arm, Ok(ApplyEmergencyResponse::Accepted))
                .unwrap();
            *frame.lock().unwrap() = input(true, 1.0);
            runtime.advance_to(Duration::from_millis(60)).unwrap();
            assert_eq!(runtime.intent().unwrap().linear_x_mps, 0.5);
            *frame.lock().unwrap() = lost;
            runtime.advance_to(Duration::from_millis(79)).unwrap();
            assert_eq!(
                runtime.intent().unwrap().linear_x_mps,
                0.5,
                "external changes wait for normal cadence"
            );
            runtime.advance_to(Duration::from_millis(80)).unwrap();
            assert!(runtime.intent().is_none());
            let cleanup = runtime
                .take_request::<Empty, ApplyEmergencyResponse>("disarm_motion")
                .unwrap()
                .unwrap();
            runtime
                .complete_request(&cleanup, Ok(ApplyEmergencyResponse::Accepted))
                .unwrap();
            let mut reconnect = input(true, 0.0);
            reconnect.devices[0].key.incarnation = 1;
            *frame.lock().unwrap() = reconnect;
            runtime.advance_to(Duration::from_millis(100)).unwrap();
            assert_eq!(runtime.status().unwrap().phase, Phase::ReleaseRequired);
            assert!(runtime.intent().is_none());
            *frame.lock().unwrap() = input(false, 0.0);
            runtime.advance_to(Duration::from_millis(120)).unwrap();
            *frame.lock().unwrap() = input(true, 0.0);
            runtime.advance_to(Duration::from_millis(160)).unwrap();
            assert!(
                runtime
                    .take_request::<ArmRequest, ApplyEmergencyResponse>("arm_motion")
                    .unwrap()
                    .is_some()
            );
        }
    }
}

#[cfg(test)]
#[path = "runtime/fixture.rs"]
mod fixture;
