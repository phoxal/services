//! The rover brain: no mission selected at startup. Both mission intents
//! are leased projections over the shared SDK MotionSetpoint payload,
//! so the brain publishes the same producer-independent velocity vocabulary
//! that Motion admits.

use phoxal::contracts::Latest;
use phoxal::contracts::robotics::MotionSetpoint;

/// The brain's endpoint contract: one leased manual intent and one leased
/// autonomous intent, both typed by the shared SDK payload.
#[phoxal::endpoints]
pub(crate) struct BrainApi {
    /// No operator mission is selected at startup.
    #[phoxal::output(projection = state, lease_ms = 100, max_bytes = 256)]
    manual: Latest<MotionSetpoint>,

    /// No autonomous mission is selected at startup.
    #[phoxal::output(projection = state, lease_ms = 100, max_bytes = 256)]
    autonomous: Latest<MotionSetpoint>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Brain;

#[phoxal::runtime(
    contract = BrainApi,
    period_ms = 20,
    timeout_ms = 100,
    init_timeout_ms = 1_000
)]
impl Brain {
    #[init]
    fn new(_config: ()) -> phoxal::Result<Self> {
        Ok(Self)
    }

    /// No operator mission is selected at startup.
    #[publish(manual)]
    fn manual(&self) -> Option<MotionSetpoint> {
        None
    }

    /// No autonomous mission is selected at startup.
    #[publish(autonomous)]
    fn autonomous(&self) -> Option<MotionSetpoint> {
        None
    }
}
