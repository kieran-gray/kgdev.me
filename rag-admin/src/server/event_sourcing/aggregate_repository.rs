use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::application::AppError;

use super::aggregate::Aggregate;
use super::event_store::EventStore;

/// A persisted snapshot of an aggregate at a given version (= event count).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateSnapshot<A> {
    pub stream_id: Uuid,
    pub version: i64,
    pub aggregate: A,
}

/// Persists and retrieves aggregate snapshots so that command handling does
/// not need to replay the full event stream every time.
#[async_trait]
pub trait SnapshotStore<A>: Send + Sync
where
    A: Aggregate,
{
    async fn load(&self, stream_id: Uuid) -> Result<Option<AggregateSnapshot<A>>, AppError>;

    async fn save(&self, snapshot: &AggregateSnapshot<A>) -> Result<(), AppError>;
}

/// Loads aggregates by combining a snapshot (if any) with the tail of events
/// written since that snapshot. Mirrors the aggregate caching pattern from
/// fern-labour's event_sourcing crate.
pub struct AggregateRepository<A>
where
    A: Aggregate,
{
    event_store: Arc<dyn EventStore<A::Event>>,
    snapshot_store: Arc<dyn SnapshotStore<A>>,
}

impl<A> AggregateRepository<A>
where
    A: Aggregate,
{
    pub fn new(
        event_store: Arc<dyn EventStore<A::Event>>,
        snapshot_store: Arc<dyn SnapshotStore<A>>,
    ) -> Self {
        Self {
            event_store,
            snapshot_store,
        }
    }

    pub async fn load(&self, stream_id: Uuid) -> Result<Option<LoadedAggregate<A>>, AppError> {
        let snapshot = self.snapshot_store.load(stream_id).await?;
        let from_sequence = snapshot.as_ref().map(|s| s.version).unwrap_or(0);
        let envelopes = self
            .event_store
            .load_after(stream_id, from_sequence)
            .await?;

        let new_event_count = envelopes.len();
        let last_sequence = envelopes
            .last()
            .map(|e| e.metadata.sequence)
            .unwrap_or(from_sequence);

        let state = match snapshot {
            Some(snap) => {
                let mut state = snap.aggregate;
                for env in &envelopes {
                    state.apply(&env.event);
                }
                Some(state)
            }
            None => {
                if envelopes.is_empty() {
                    None
                } else {
                    let events: Vec<_> = envelopes.iter().map(|e| e.event.clone()).collect();
                    A::from_events(&events)
                }
            }
        };

        Ok(state.map(|aggregate| LoadedAggregate {
            stream_id,
            version: last_sequence,
            new_events_since_snapshot: new_event_count,
            aggregate,
        }))
    }

    /// Save a snapshot of the aggregate. Called after a successful append so
    /// the next load reads one row instead of replaying the whole stream.
    pub async fn save_snapshot(
        &self,
        stream_id: Uuid,
        version: i64,
        aggregate: &A,
    ) -> Result<(), AppError> {
        self.snapshot_store
            .save(&AggregateSnapshot {
                stream_id,
                version,
                aggregate: aggregate.clone(),
            })
            .await
    }

    pub fn event_store(&self) -> Arc<dyn EventStore<A::Event>> {
        self.event_store.clone()
    }
}

/// Aggregate state plus the bookkeeping the command processor needs: the
/// stream version (for optimistic concurrency on append) and how many events
/// have accumulated since the last snapshot (so we can decide when to re-snap).
pub struct LoadedAggregate<A> {
    pub stream_id: Uuid,
    pub version: i64,
    pub new_events_since_snapshot: usize,
    pub aggregate: A,
}
