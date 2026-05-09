use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::ChunkingConfig;

use super::status::IngestStage;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngestRequested {
    pub document_id: Uuid,
    pub pipeline_configuration_id: Uuid,
    pub document_version: u32,
    pub chunking_config: ChunkingConfig,
    pub request_id: Uuid,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkingCompleted {
    pub chunk_set_id: Uuid,
    pub chunk_count: u32,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmbeddingCompleted {
    pub embedding_set_id: Uuid,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexingCompleted {
    pub vector_count: u32,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngestionFailed {
    pub stage: IngestStage,
    pub reason: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngestionRetried {
    pub request_id: Uuid,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexingRemoved {
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum IndexingEvent {
    IngestRequested(IngestRequested),
    ChunkingCompleted(ChunkingCompleted),
    EmbeddingCompleted(EmbeddingCompleted),
    IndexingCompleted(IndexingCompleted),
    IngestionFailed(IngestionFailed),
    IngestionRetried(IngestionRetried),
    IndexingRemoved(IndexingRemoved),
}
