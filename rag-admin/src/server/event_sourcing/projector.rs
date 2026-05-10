use async_trait::async_trait;

use crate::server::application::AppError;

use super::envelope::EventEnvelope;

/// Builds a read model by folding events into row-level INSERT/UPDATE/DELETE.
///
/// Each projector owns its read model table and tracks its own checkpoint.
/// Projectors are driven by `ProjectionDriver` from the event log; one bad
/// projector cannot block another (they are scheduled independently and
/// faulted-after-N).
#[async_trait]
pub trait Projector<E>: Send + Sync
where
    E: Send + Sync,
{
    /// Stable name used as the checkpoint key. Changing it resets the projector.
    fn name(&self) -> &str;

    /// Apply a contiguous batch of events to the read model. The driver guarantees
    /// the batch is in `log_position` order and starts at the projector's checkpoint.
    async fn project(&self, events: &[EventEnvelope<E>]) -> Result<(), AppError>;
}
