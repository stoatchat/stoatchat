use std::time::Duration;

use crate::{revolt_result::Result, RatelimitEvent, RatelimitEventType};

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractRatelimitEvents: Sync + Send {
    /// Insert a new ratelimit event
    async fn insert_ratelimit_event(&self, event: &RatelimitEvent) -> Result<()>;

    /// Count number of events in given duration and check if we've hit the limit
    async fn has_ratelimited(
        &self,
        target_id: &str,
        event_type: RatelimitEventType,
        period: Duration,
        count: usize,
    ) -> Result<bool>;
}
