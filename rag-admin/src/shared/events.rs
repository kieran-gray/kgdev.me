use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Client-side mirror of `crate::server::event_sourcing::envelope::PublishedEvent`.
///
/// The server-side type uses domain newtypes (`Timestamp`) that are gated behind
/// the `ssr` feature. This DTO inlines the wire shape so it can be deserialized
/// on the hydrate target without dragging the server domain in. The two types
/// must stay in sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedEvent {
    pub stream_id: Uuid,
    pub aggregate_type: String,
    pub sequence: i64,
    pub log_position: i64,
    pub event_type: String,
    pub event_data: serde_json::Value,
    pub occurred_at: String,
}

/// Aggregate-type discriminator strings as published by the server.
///
/// These must match the values returned by each aggregate's
/// `Aggregate::aggregate_type()` impl. They are part of the wire contract for
/// the event-bus websocket — bumping them is a coordinated server+client
/// change.
pub mod aggregate {
    pub const SOURCE_DOCUMENT: &str = "source_document";
    pub const INDEXING: &str = "indexing";
    pub const CONFIGURATION: &str = "configuration";
    pub const EVALUATION_DATASET: &str = "evaluation_dataset";
    pub const EVALUATION_RUN: &str = "evaluation_run";
}

impl PublishedEvent {
    /// Convenience: did this event come from one of the named aggregate types?
    pub fn from_any(&self, aggregate_types: &[&str]) -> bool {
        aggregate_types
            .iter()
            .any(|t| *t == self.aggregate_type.as_str())
    }
}
