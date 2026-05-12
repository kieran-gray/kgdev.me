use uuid::Uuid;

use crate::server::domain::shared::Timestamp;
use crate::shared::ChunkingConfig;

use super::status::IngestStage;

pub struct RequestIngest {
    pub document_id: Uuid,
    pub pipeline_configuration_id: Uuid,
    pub document_version: u32,
    pub chunking_config: ChunkingConfig,
    pub request_id: Uuid,
    /// When `true`, the process manager chains stages: `ChunkingCompleted`
    /// fires the embedding effect, `EmbeddingCompleted` fires the indexing
    /// effect. When `false`, the chain stops after each stage and the
    /// operator must `RequeueX` to continue.
    pub auto_advance: bool,
    pub occurred_at: Timestamp,
}

pub struct CompleteChunking {
    pub chunk_set_id: Uuid,
    pub chunk_count: u32,
    pub occurred_at: Timestamp,
}

pub struct CompleteEmbedding {
    pub embedding_set_id: Uuid,
    pub occurred_at: Timestamp,
}

pub struct CompleteIndexing {
    pub vector_count: u32,
    pub occurred_at: Timestamp,
}

pub struct FailIngestion {
    pub stage: IngestStage,
    pub reason: String,
    pub occurred_at: Timestamp,
}

pub struct RetryIngestion {
    pub request_id: Uuid,
    pub occurred_at: Timestamp,
}

pub struct RemoveIndexing {
    pub occurred_at: Timestamp,
}

/// Operator-triggered "run chunking now". Emits a marker event the policy
/// turns into a `ChunkingEffect`; does not change aggregate state.
pub struct RequeueChunking {
    pub occurred_at: Timestamp,
}

/// Operator-triggered "run embedding now". Requires the chunk set to exist.
pub struct RequeueEmbedding {
    pub occurred_at: Timestamp,
}

/// Operator-triggered "run indexing (upsert) now". Requires the embedding
/// set to exist.
pub struct RequeueIndexing {
    pub occurred_at: Timestamp,
}

pub enum IndexingCommand {
    RequestIngest(RequestIngest),
    CompleteChunking(CompleteChunking),
    CompleteEmbedding(CompleteEmbedding),
    CompleteIndexing(CompleteIndexing),
    FailIngestion(FailIngestion),
    RetryIngestion(RetryIngestion),
    RemoveIndexing(RemoveIndexing),
    RequeueChunking(RequeueChunking),
    RequeueEmbedding(RequeueEmbedding),
    RequeueIndexing(RequeueIndexing),
}
