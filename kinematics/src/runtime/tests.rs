use phoxal::runtime::{ExecutionDuration, ExecutionTime, Harness, ObservationStamp, StepContext};

use super::*;
type ServiceInputs = <crate::contract::KinematicsApi as phoxal::runtime::RuntimeContract>::Inputs;

fn config() -> KinematicsConfig {
    KinematicsConfig::default()
}

fn new_service(config: KinematicsConfig) -> Harness<Kinematics> {
    Harness::new(config).expect("valid configuration")
}

struct JointBatch {
    joints: Vec<crate::contract::JointState>,
}

fn step(
    mut service: Harness<Kinematics>,
    context: &StepContext,
    inputs: &ServiceInputs,
) -> (Harness<Kinematics>, JointBatch) {
    let now = context.now().as_nanos();
    if now > 0 {
        service
            .advance_to(std::time::Duration::from_nanos(now - 1))
            .expect("preceding declared releases");
    }
    for sample in inputs.encoders.items() {
        service
            .inject_encoders(Sample::new(*sample.payload(), sample.stamp().clone()))
            .expect("bounded captured input");
    }
    service
        .advance_to(std::time::Duration::from_nanos(now))
        .expect("kinematics accepts its next declared release");
    let joints = service.joints();
    (service, JointBatch { joints })
}

fn sample(
    encoder_id: &str,
    position_rad: f64,
    velocity_radps: f64,
    at_ms: u64,
) -> Sample<EncoderSample> {
    Sample::new(
        EncoderSample {
            position_rad: Some(position_rad),
            velocity_radps: Some(velocity_radps),
        },
        ObservationStamp::new(
            encoder_id,
            ExecutionTime::from_nanos(at_ms * 1_000_000),
            None,
        ),
    )
}

fn context(index: u64, now_ms: u64, previous_ms: Option<u64>) -> StepContext {
    StepContext::from_previous(
        ExecutionTime::from_nanos(now_ms * 1_000_000),
        ExecutionDuration::from_millis(20),
        previous_ms.map(|at| ExecutionTime::from_nanos(at * 1_000_000)),
        0,
        index,
    )
}

#[test]
fn one_encoder_batch_produces_joints_odometry_and_frames() {
    let service = new_service(config());
    let inputs = ServiceInputs {
        encoders: Samples::new(vec![
            sample("left_encoder", 2.0, 4.0, 20),
            sample("right_encoder", 2.0, 4.0, 20),
        ]),
        lookup_frame: Default::default(),
    };
    let (service, outputs) = step(service, &context(0, 20, None), &inputs);
    assert_eq!(outputs.joints.len(), 2);
    assert!(
        outputs
            .joints
            .iter()
            .any(|joint| joint.joint_id == "left_wheel" && joint.position_rad == 2.0)
    );
    assert!(service.odometry().expect("accepted odometry").available);
    assert_eq!(service.odometry().expect("accepted odometry").revision, 1);
    assert!(validation::frame_tree(&service.frames().expect("accepted frames")).is_ok());
}

#[test]
fn expired_wheel_is_explicitly_unavailable_and_does_not_integrate() {
    let service = new_service(config());
    let (service, _) = step(
        service,
        &context(0, 20, None),
        &ServiceInputs {
            encoders: Samples::new(vec![
                sample("left_encoder", 0.0, 4.0, 20),
                sample("right_encoder", 0.0, 4.0, 20),
            ]),
            lookup_frame: Default::default(),
        },
    );
    let mut service = service;
    service
        .advance_to(std::time::Duration::from_millis(120))
        .expect("last fresh periodic release");
    let last_fresh = service.odometry().expect("fresh odometry");
    assert!(last_fresh.available);
    assert_eq!(last_fresh.revision, 6);
    let (service, _) = step(
        service,
        &context(6, 140, Some(120)),
        &ServiceInputs {
            encoders: Samples::new(vec![sample("left_encoder", 0.0, 4.0, 140)]),
            lookup_frame: Default::default(),
        },
    );
    assert!(!service.odometry().expect("accepted odometry").available);
    assert_eq!(
        service.odometry().expect("accepted odometry").linear_x_mps,
        0.0
    );
    assert_eq!(
        service
            .odometry()
            .expect("accepted odometry")
            .angular_z_radps,
        0.0
    );
    let expired = service.odometry().expect("expired odometry");
    assert_eq!(expired.revision, last_fresh.revision);
    assert_eq!(expired.x_m, last_fresh.x_m);
    assert_eq!(expired.y_m, last_fresh.y_m);
    assert_eq!(expired.yaw_rad, last_fresh.yaw_rad);
}

#[test]
fn stale_measurements_are_not_reused_as_fresh_motion() {
    let service = new_service(config());
    let (service, outputs) = step(
        service,
        &context(0, 200, None),
        &ServiceInputs {
            encoders: Samples::new(vec![
                sample("left_encoder", 0.0, 4.0, 20),
                sample("right_encoder", 0.0, 4.0, 20),
            ]),
            lookup_frame: Default::default(),
        },
    );
    assert!(outputs.joints.is_empty());
    assert!(!service.odometry().expect("accepted odometry").available);
    assert_eq!(
        service.odometry().expect("accepted odometry").linear_x_mps,
        0.0
    );
}

#[test]
fn four_wheels_use_every_calibration_and_preserve_the_oldest_capture() {
    let mut cfg = config();
    cfg.left_wheels.push(crate::config::WheelEncoder {
        encoder_id: "left_rear".into(),
        joint_id: "left_rear_wheel".into(),
        longitudinal_offset_m: -0.18,
        direction_sign: -1,
        gear_ratio: 2.0,
    });
    cfg.right_wheels.push(crate::config::WheelEncoder {
        encoder_id: "right_rear".into(),
        joint_id: "right_rear_wheel".into(),
        longitudinal_offset_m: -0.18,
        direction_sign: -1,
        gear_ratio: 2.0,
    });
    validate_config(&cfg).unwrap();
    let service = new_service(cfg);
    let (service, outputs) = step(
        service,
        &context(0, 20, Some(0)),
        &ServiceInputs {
            encoders: Samples::new(vec![
                sample("left_encoder", 0.0, 2.0, 20),
                sample("left_rear", 0.0, -8.0, 10),
                sample("right_encoder", 0.0, 4.0, 20),
                sample("right_rear", 0.0, -12.0, 20),
            ]),
            lookup_frame: Default::default(),
        },
    );
    assert!(service.odometry().expect("accepted odometry").available);
    assert_eq!(outputs.joints.len(), 4);
    assert!((service.odometry().expect("accepted odometry").linear_x_mps - 0.4).abs() < 1e-12);
    assert!(
        (service
            .odometry()
            .expect("accepted odometry")
            .angular_z_radps
            - 0.5)
            .abs()
            < 1e-12
    );
    assert_eq!(
        service
            .odometry()
            .expect("accepted odometry")
            .oldest_capture_time_nanos,
        Some(10_000_000)
    );
    // Exact constant-twist integration over 20 ms: radius .8 m, angle .01 rad.
    assert!(
        (service.odometry().expect("accepted odometry").x_m - 0.8 * 0.01_f64.sin()).abs() < 1e-12
    );
    assert!(
        (service.odometry().expect("accepted odometry").y_m - 0.8 * (1.0 - 0.01_f64.cos())).abs()
            < 1e-12
    );
    assert_eq!(
        service.frames().expect("accepted frames").transforms.len(),
        5
    );
    let rear = service
        .frames()
        .expect("accepted frames")
        .transforms
        .into_iter()
        .find(|frame| frame.child_frame_id == "left_rear_wheel")
        .unwrap();
    assert_eq!((rear.x_m, rear.y_m), (-0.18, 0.2));
    let (service, outputs) = step(
        service,
        &context(1, 40, Some(20)),
        &ServiceInputs {
            encoders: Samples::default(),
            lookup_frame: Default::default(),
        },
    );
    assert!(
        service.odometry().expect("accepted odometry").available,
        "slower captures remain usable within their original age bound"
    );
    assert_eq!(
        service
            .odometry()
            .expect("accepted odometry")
            .oldest_capture_time_nanos,
        Some(10_000_000)
    );
    assert!(
        outputs.joints.is_empty(),
        "retention does not republish measured samples"
    );
    let (service, _) = step(
        service,
        &context(2, 120, Some(40)),
        &ServiceInputs {
            encoders: Samples::default(),
            lookup_frame: Default::default(),
        },
    );
    assert!(
        !service.odometry().expect("accepted odometry").available,
        "the oldest rear wheel expires before the other three"
    );
}

#[test]
fn invalid_new_encoder_replaces_old_evidence_until_a_new_valid_capture() {
    let service = new_service(config());
    let (service, _) = step(
        service,
        &context(0, 20, None),
        &ServiceInputs {
            encoders: Samples::new(vec![
                sample("left_encoder", 0.0, 1.0, 20),
                sample("right_encoder", 0.0, 1.0, 20),
            ]),
            lookup_frame: Default::default(),
        },
    );
    let (service, _) = step(
        service,
        &context(1, 40, Some(20)),
        &ServiceInputs {
            encoders: Samples::new(vec![sample("left_encoder", 0.0, f64::NAN, 40)]),
            lookup_frame: Default::default(),
        },
    );
    assert!(!service.odometry().expect("accepted odometry").available);
    let (service, _) = step(
        service,
        &context(2, 60, Some(40)),
        &ServiceInputs {
            encoders: Samples::default(),
            lookup_frame: Default::default(),
        },
    );
    assert!(
        !service.odometry().expect("accepted odometry").available,
        "a quiet interval cannot revive the older valid sample"
    );
    let (service, _) = step(
        service,
        &context(3, 80, Some(60)),
        &ServiceInputs {
            encoders: Samples::new(vec![sample("left_encoder", 0.0, 1.0, 80)]),
            lookup_frame: Default::default(),
        },
    );
    assert!(service.odometry().expect("accepted odometry").available);
    assert_eq!(
        service
            .odometry()
            .expect("accepted odometry")
            .oldest_capture_time_nanos,
        Some(20_000_000)
    );
}

fn lookup(revision: u64) -> LookupFrameRequest {
    LookupFrameRequest {
        parent_frame_id: "odom".into(),
        child_frame_id: "base_link".into(),
        revision,
    }
}

#[test]
fn lookup_handler_integrates_once_and_retention_does_not_reemit_joint_samples() {
    let mut service = new_service(config());
    assert_eq!(service.odometry().unwrap().revision, 0);
    assert!(service.joints().is_empty());
    for side in ["left_encoder", "right_encoder"] {
        service.inject_encoders(sample(side, 1.0, 4.0, 0)).unwrap();
    }
    let call = service.enqueue_lookup_frame(lookup(0)).unwrap();
    service.advance_to(std::time::Duration::ZERO).unwrap();
    assert_eq!(service.reply(call).unwrap().revision, 1);
    assert_eq!(service.odometry().unwrap().revision, 1);
    assert_eq!(service.joints().len(), 2);
    service
        .advance_to(std::time::Duration::from_millis(19))
        .unwrap();
    assert_eq!(service.odometry().unwrap().revision, 1);
    service
        .advance_to(std::time::Duration::from_millis(20))
        .unwrap();
    assert!(service.joints().is_empty());
    assert!((service.odometry().unwrap().x_m - 0.008).abs() < 1e-12);
}

#[test]
fn lookup_current_history_unknown_eviction_and_reset() {
    let mut cfg = config();
    cfg.history_capacity = 2;
    let mut service = new_service(cfg.clone());
    for side in ["left_encoder", "right_encoder"] {
        service.inject_encoders(sample(side, 1.0, 4.0, 0)).unwrap();
    }
    service.advance_to(std::time::Duration::ZERO).unwrap();
    let old = service.enqueue_lookup_frame(lookup(1)).unwrap();
    service
        .advance_to(std::time::Duration::from_millis(20))
        .unwrap();
    assert_eq!(service.reply(old).unwrap().transform.unwrap().x_m, 0.0);
    let old = service.enqueue_lookup_frame(lookup(1)).unwrap();
    let current = service.enqueue_lookup_frame(lookup(0)).unwrap();
    let mut missing = lookup(0);
    missing.child_frame_id = "missing".into();
    let missing = service.enqueue_lookup_frame(missing).unwrap();
    service
        .advance_to(std::time::Duration::from_millis(40))
        .unwrap();
    assert!(service.reply(old).unwrap().transform.is_none());
    assert_eq!(service.reply(current).unwrap().revision, 3);
    assert!(service.reply(missing).unwrap().transform.is_none());
    service.reset(cfg).unwrap();
    assert_eq!(service.odometry().unwrap().revision, 0);
    assert_eq!(service.odometry().unwrap().x_m, 0.0);
    assert!(!service.odometry().unwrap().available);
    service
        .advance_to(std::time::Duration::from_millis(40))
        .unwrap();
    assert!(service.joints().is_empty());
    assert!(!service.odometry().unwrap().available);
}
