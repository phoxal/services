//! Read-only physical qualification, outside the participant runtime.
#[allow(
    dead_code,
    reason = "the observer uses only the owner's status contract"
)]
#[path = "../../../../../motion/src/contract.rs"]
mod motion;
use phoxal::session::{ConnectionConfig, ObservationItem, connect};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: public-observer-fixture ENDPOINT DURATION_SECONDS".into());
    }
    let duration = args[1].parse::<u64>()?;
    if !(1..=600).contains(&duration) {
        return Err("duration must be between 1 and 600 seconds".into());
    }
    let connection = connect(ConnectionConfig::new(
        &args[0],
        "local",
        "physical-qualification-observer",
    )?)
    .await?;
    let supervisor = connection.supervisor("local").await?;
    let executions = supervisor.management().executions().await?;
    if executions.len() != 1 {
        return Err("expected exactly one execution".into());
    }
    let execution = supervisor.execution(&executions[0].execution_id).await?;
    let service = execution.service("motion").await?;
    let status = service.method(motion::MotionApi::STATUS).await?;
    let mut stream = status.observe().await?;
    let started = std::time::Instant::now();
    println!(
        "read-only observer execution={} duration={}s",
        executions[0].execution_id, duration
    );
    let mut previous = None;
    let result = tokio::time::timeout(Duration::from_secs(duration), async {
        while let Some(item) = stream.recv().await {
            match item.map_err(|error| error.to_string())? {
                ObservationItem::Value { revision, value } => {
                    let state = format!(
                        "mode={:?} owner={:?} stopped={} emergency={} protective_clear={}",
                        value.mode,
                        value.selected_owner_id,
                        value.stopped,
                        value.emergency_latched,
                        value.protective_state_clear
                    );
                    if previous.as_ref() != Some(&state) {
                        println!(
                            "wall_ms={} revision={} {}",
                            started.elapsed().as_millis(),
                            revision,
                            state
                        );
                        previous = Some(state);
                    }
                }
                ObservationItem::InitialAbsent { revision } => {
                    println!("initial absent revision={revision}")
                }
                ObservationItem::Gap { revision, dropped } => {
                    return Err(format!(
                        "observation gap revision={revision} dropped={dropped}"
                    ));
                }
                ObservationItem::Failed { detail, .. } => return Err(detail),
                ObservationItem::End { revision } => {
                    println!("stream ended revision={revision}");
                    break;
                }
            }
        }
        Ok::<(), String>(())
    })
    .await;
    drop(stream);
    supervisor.close().await?;
    connection.close().await?;
    match result {
        Ok(result) => result.map_err(Into::into),
        Err(_) => Ok(()),
    }
}
