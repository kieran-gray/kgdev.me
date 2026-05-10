use async_trait::async_trait;
use uuid::Uuid;

use crate::server::application::AppError;

use super::envelope::EventEnvelope;

/// Result of a successful append: the events as they were written, with their
/// assigned sequence and log_position.
pub type AppendedEvent<E> = EventEnvelope<E>;

/// Persistent, ordered log of events for a single aggregate type.
///
/// One trait impl per `E`. Backed by Postgres `events` table, filtered by
/// `aggregate_type`.
#[async_trait]
pub trait EventStore<E>: Send + Sync
where
    E: Send + Sync,
{
    /// Load all events for one aggregate stream, ordered by `sequence`.
    async fn load(&self, stream_id: Uuid) -> Result<Vec<EventEnvelope<E>>, AppError>;

    /// Load events for one aggregate stream after a given sequence (exclusive).
    /// Used by `AggregateRepository` to replay only events newer than the snapshot.
    async fn load_after(
        &self,
        stream_id: Uuid,
        after_sequence: i64,
    ) -> Result<Vec<EventEnvelope<E>>, AppError>;

    /// Append events to a stream with optimistic concurrency. `expected_version`
    /// is the count of events currently in the stream; mismatch is a conflict.
    async fn append(
        &self,
        stream_id: Uuid,
        expected_version: usize,
        events: &[E],
    ) -> Result<Vec<AppendedEvent<E>>, AppError>;

    /// Load events globally for this aggregate type with `log_position > after`,
    /// ordered ascending. Used by the projection driver to advance projectors.
    async fn load_global_after(
        &self,
        after_log_position: i64,
        limit: i64,
    ) -> Result<Vec<EventEnvelope<E>>, AppError>;
}
