use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::domain::shared::Timestamp;

/// Metadata attached to every persisted event, derived from the event store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventMetadata {
    pub stream_id: Uuid,
    pub aggregate_type: String,
    pub sequence: i64,
    pub log_position: i64,
    pub event_type: String,
    pub occurred_at: Timestamp,
}

/// An event together with its persistence metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope<E> {
    pub event: E,
    pub metadata: EventMetadata,
}

/// Cross-aggregate-type event payload published on the in-process event bus.
///
/// This is what reaches WebSocket subscribers. The payload is pre-serialised so
/// the bus is monomorphic across aggregate types and clients can treat it as
/// an opaque cache invalidation signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedEvent {
    pub stream_id: Uuid,
    pub aggregate_type: String,
    pub sequence: i64,
    pub log_position: i64,
    pub event_type: String,
    pub event_data: serde_json::Value,
    pub occurred_at: Timestamp,
}

impl<E: Serialize> EventEnvelope<E> {
    pub fn to_published(&self) -> Result<PublishedEvent, serde_json::Error> {
        Ok(PublishedEvent {
            stream_id: self.metadata.stream_id,
            aggregate_type: self.metadata.aggregate_type.clone(),
            sequence: self.metadata.sequence,
            log_position: self.metadata.log_position,
            event_type: self.metadata.event_type.clone(),
            event_data: serde_json::to_value(&self.event)?,
            occurred_at: self.metadata.occurred_at.clone(),
        })
    }
}
