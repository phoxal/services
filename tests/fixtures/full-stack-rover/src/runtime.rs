//! The rover brain: no mission selected at startup. Both mission intents
//! are leased setpoint projections over the selected Motion service's
//! generated intent payload, so the brain publishes exactly the leased
//! intents Motion admits.

use phoxal::contracts::Latest;

/// The brain's endpoint contract: one leased manual intent and one leased
/// autonomous intent, both typed by Motion's generated payload.
#[phoxal::endpoints]
pub(crate) struct BrainApi {
    /// No operator mission is selected at startup.
    #[phoxal::output(projection = state, lease_ms = 100, max_bytes = 256)]
    manual: Latest<crate::api::motion::MotionIntent>,

    /// No autonomous mission is selected at startup.
    #[phoxal::output(projection = state, lease_ms = 100, max_bytes = 256)]
    autonomous: Latest<crate::api::motion::MotionIntent>,
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
    fn manual(&self) -> Option<crate::api::motion::MotionIntent> {
        None
    }

    /// No autonomous mission is selected at startup.
    #[publish(autonomous)]
    fn autonomous(&self) -> Option<crate::api::motion::MotionIntent> {
        None
    }
}
