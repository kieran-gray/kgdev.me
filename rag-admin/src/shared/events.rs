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

impl PublishedEvent {
    /// Convenience: did this event come from one of the named aggregate types?
    pub fn from_any(&self, aggregate_types: &[&str]) -> bool {
        aggregate_types
            .iter()
            .any(|t| *t == self.aggregate_type.as_str())
    }
}
