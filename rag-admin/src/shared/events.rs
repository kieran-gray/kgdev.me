use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

pub mod aggregate {
    pub const SOURCE_DOCUMENT: &str = "source_document";
    pub const INDEXING: &str = "indexing";
    pub const CONFIGURATION: &str = "configuration";
    pub const EVALUATION_DATASET: &str = "evaluation_dataset";
    pub const EVALUATION_RUN: &str = "evaluation_run";
}

impl PublishedEvent {
    pub fn from_any(&self, aggregate_types: &[&str]) -> bool {
        aggregate_types.contains(&self.aggregate_type.as_str())
    }
}
