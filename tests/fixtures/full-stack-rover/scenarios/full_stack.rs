//! Native movement check through the rover's selected Motion service.

use phoxal::scenario::{CapturePolicy, Simulation};
phoxal::api!();

use api::motion;
use api::motion::ControlMode;
use api::motion::{ApplyEmergencyResponse, ArmRequest};
use api::{kinematics, safety};
use phoxal::contracts::robotics::MotionSetpoint;

fn forward_turn_stop(sim: &mut Simulation) -> phoxal::Result<()> {
    let mut plan = sim.plan();
    let body = plan.record_body("robot-rover")?;
    let navigation = plan.record(
        api::navigation::status(),
        CapturePolicy::best_effort_history(1_024)?,
    )?;
    let wheels = [
        (
            "front_left_drive",
            plan.record(
                api::front_left_drive::encoder(),
                CapturePolicy::best_effort_history(1_024)?,
            )?,
        ),
        (
            "front_right_drive",
            plan.record(
                api::front_right_drive::encoder(),
                CapturePolicy::best_effort_history(1_024)?,
            )?,
        ),
        (
            "rear_left_drive",
            plan.record(
                api::rear_left_drive::encoder(),
                CapturePolicy::best_effort_history(1_024)?,
            )?,
        ),
        (
            "rear_right_drive",
            plan.record(
                api::rear_right_drive::encoder(),
                CapturePolicy::best_effort_history(1_024)?,
            )?,
        ),
    ];

    let status = plan.record(motion::status(), CapturePolicy::best_effort_history(1_024)?)?;
    let safety_status =
        plan.record(safety::status(), CapturePolicy::best_effort_history(1_024)?)?;
    let odometry = plan.record(
        kinematics::odometry(),
        CapturePolicy::best_effort_history(1_024)?,
    )?;
    plan.wait_steps(50)?;
    plan.send(manual(0.5, 0.0))?;
    plan.wait_steps(1)?;
    let arm = plan.send(motion::arm(ArmRequest {
        mode: ControlMode::Manual,
    }))?;
    plan.wait_steps(1)?;
    for _ in 0..50 {
        plan.send(manual(0.5, 0.0))?;
        plan.wait_steps(3)?;
    }
    for _ in 0..35 {
        plan.send(manual(0.0, 1.5))?;
        plan.wait_steps(3)?;
    }
    plan.send(manual(0.0, 0.0))?;
    plan.wait_steps(20)?;
    plan.send(motion::withdraw_manual())?;
    let disarm = plan.send(motion::disarm(phoxal::contracts::Empty {}))?;
    plan.wait_steps(20)?;

    let observed = sim.run(plan)?;
    assert!(
        observed.history(&navigation)?.iter().any(|sample| {
            sample
                .value()
                .map_revision
                .is_some_and(|revision| revision > 0)
        }),
        "Navigation never observed the converted World revision"
    );
    for (wheel, capture) in wheels {
        let moved = observed.history(&capture)?.iter().any(|sample| {
            sample
                .value()
                .velocity_radps
                .is_some_and(|value| value.abs() > 0.1)
        });
        assert!(moved, "{wheel} never observed wheel motion");
    }

    let statuses = observed.history(&status)?;
    let safety_history = observed.history(&safety_status)?;
    let odometry_history = observed.history(&odometry)?;
    let arm = observed.reply(arm)?;
    let disarm = observed.reply(disarm)?;
    assert!(
        matches!(arm, ApplyEmergencyResponse::Accepted),
        "Motion refused Arm: {arm:?}; Safety status: {:?}; odometry: {:?}",
        safety_history.last().map(|value| value.value()),
        odometry_history.last().map(|value| value.value()),
    );
    assert!(
        matches!(disarm, ApplyEmergencyResponse::Accepted),
        "Motion refused Disarm: {disarm:?}"
    );
    let final_status = statuses
        .last()
        .ok_or_else(|| phoxal::anyhow!("no Motion status"))?
        .value();
    assert_eq!(final_status.mode, ControlMode::Disarmed);
    assert!(final_status.stopped, "Motion status did not report a stop");
    let body = observed.body_history(&body)?;
    let first = body
        .first()
        .ok_or_else(|| phoxal::anyhow!("body history is empty"))?
        .value();
    let last = body
        .last()
        .ok_or_else(|| phoxal::anyhow!("body history is empty"))?
        .value();
    let displacement = ((last.position_m[0] - first.position_m[0]).powi(2)
        + (last.position_m[1] - first.position_m[1]).powi(2))
    .sqrt();
    assert!(
        displacement >= 0.5,
        "native displacement {displacement:.3} m is too small"
    );
    let speed = last
        .linear_velocity_mps
        .iter()
        .map(|value| value.powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(speed < 0.03, "rover did not stop: {speed:.3} m/s");
    assert!(
        last.angular_velocity_radps[2].abs() < 0.05,
        "rover did not stop rotating: {:.3} rad/s",
        last.angular_velocity_radps[2]
    );
    Ok(())
}

fn four_wheel_qualification(sim: &mut Simulation) -> phoxal::Result<()> {
    let mut plan = sim.plan();
    let body = plan.record_body("robot-rover")?;
    let navigation = plan.record(
        api::navigation::status(),
        CapturePolicy::best_effort_history(1_024)?,
    )?;
    let wheels = [
        (
            "front_left_drive",
            plan.record(
                api::front_left_drive::encoder(),
                CapturePolicy::best_effort_history(1_024)?,
            )?,
        ),
        (
            "front_right_drive",
            plan.record(
                api::front_right_drive::encoder(),
                CapturePolicy::best_effort_history(1_024)?,
            )?,
        ),
        (
            "rear_left_drive",
            plan.record(
                api::rear_left_drive::encoder(),
                CapturePolicy::best_effort_history(1_024)?,
            )?,
        ),
        (
            "rear_right_drive",
            plan.record(
                api::rear_right_drive::encoder(),
                CapturePolicy::best_effort_history(1_024)?,
            )?,
        ),
    ];

    let status = plan.record(motion::status(), CapturePolicy::best_effort_history(1_024)?)?;
    let safety_status =
        plan.record(safety::status(), CapturePolicy::best_effort_history(1_024)?)?;
    let odometry = plan.record(
        kinematics::odometry(),
        CapturePolicy::best_effort_history(1_024)?,
    )?;
    plan.wait_steps(50)?;
    plan.send(manual(0.5, 0.0))?;
    plan.wait_steps(1)?;
    let arm = plan.send(motion::arm(ArmRequest {
        mode: ControlMode::Manual,
    }))?;
    plan.wait_steps(1)?;
    for _ in 0..50 {
        plan.send(manual(0.5, 0.0))?;
        plan.wait_steps(3)?;
    }
    // Retain the internal robot's longer 320-step, 2 rad/s turn as a
    // separate qualification; the original rover scenario stays intact.
    for _ in 0..106 {
        plan.send(manual(0.0, 2.0))?;
        plan.wait_steps(3)?;
    }
    plan.send(manual(0.0, 2.0))?;
    plan.wait_steps(2)?;
    plan.send(manual(0.0, 0.0))?;
    plan.wait_steps(70)?;
    plan.send(motion::withdraw_manual())?;
    let disarm = plan.send(motion::disarm(phoxal::contracts::Empty {}))?;
    plan.wait_steps(20)?;

    let observed = sim.run(plan)?;
    assert!(
        observed.history(&navigation)?.iter().any(|sample| {
            sample
                .value()
                .map_revision
                .is_some_and(|revision| revision > 0)
        }),
        "Navigation never observed the converted World revision"
    );
    for (wheel, capture) in wheels {
        let moved = observed.history(&capture)?.iter().any(|sample| {
            sample
                .value()
                .velocity_radps
                .is_some_and(|value| value.abs() > 0.1)
        });
        assert!(moved, "{wheel} never observed wheel motion");
    }

    let statuses = observed.history(&status)?;
    let safety_history = observed.history(&safety_status)?;
    let odometry_history = observed.history(&odometry)?;
    let arm = observed.reply(arm)?;
    let disarm = observed.reply(disarm)?;
    assert!(
        matches!(arm, ApplyEmergencyResponse::Accepted),
        "Motion refused Arm: {arm:?}; Safety status: {:?}; odometry: {:?}",
        safety_history.last().map(|value| value.value()),
        odometry_history.last().map(|value| value.value()),
    );
    assert!(
        matches!(disarm, ApplyEmergencyResponse::Accepted),
        "Motion refused Disarm: {disarm:?}"
    );
    let final_status = statuses
        .last()
        .ok_or_else(|| phoxal::anyhow!("no Motion status"))?
        .value();
    assert_eq!(final_status.mode, ControlMode::Disarmed);
    assert!(final_status.stopped, "Motion status did not report a stop");
    let body = observed.body_history(&body)?;
    let first = body
        .first()
        .ok_or_else(|| phoxal::anyhow!("body history is empty"))?
        .value();
    let last = body
        .last()
        .ok_or_else(|| phoxal::anyhow!("body history is empty"))?
        .value();
    let displacement = ((last.position_m[0] - first.position_m[0]).powi(2)
        + (last.position_m[1] - first.position_m[1]).powi(2))
    .sqrt();
    assert!(
        displacement >= 0.5,
        "native displacement {displacement:.3} m is too small"
    );
    let yaw_change = body.windows(2).fold(0.0, |total, pair| {
        let mut delta =
            yaw(pair[1].value().orientation_wxyz) - yaw(pair[0].value().orientation_wxyz);
        if delta > std::f64::consts::PI {
            delta -= std::f64::consts::TAU;
        } else if delta < -std::f64::consts::PI {
            delta += std::f64::consts::TAU;
        }
        total + delta
    });
    assert!(
        yaw_change.abs() >= 1.0,
        "yaw change {yaw_change:.3} rad is below 1.0 rad"
    );
    let speed = last
        .linear_velocity_mps
        .iter()
        .map(|value| value.powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(speed < 0.03, "rover did not stop: {speed:.3} m/s");
    assert!(
        last.angular_velocity_radps[2].abs() < 0.05,
        "rover did not stop rotating: {:.3} rad/s",
        last.angular_velocity_radps[2]
    );
    Ok(())
}

fn manual(
    linear_x_mps: f64,
    angular_z_radps: f64,
) -> impl phoxal::scenario::SendOperation<Response = phoxal::contracts::Empty> {
    motion::manual(MotionSetpoint {
        linear_x_mps,
        angular_z_radps,
    })
}

fn yaw(q: [f64; 4]) -> f64 {
    let [w, x, y, z] = q;
    (2.0 * (w * z + x * y)).atan2(1.0 - 2.0 * (y * y + z * z))
}

fn main() -> phoxal::Result<()> {
    let mut simulation = Simulation::new("simulation/scene.xml")?;
    forward_turn_stop(&mut simulation)?;
    four_wheel_qualification(&mut simulation)
}
