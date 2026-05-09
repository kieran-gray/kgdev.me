use uuid::Uuid;

use crate::shared::ChunkingConfig;

use super::status::IngestStage;

pub struct RequestIngest {
    pub document_id: Uuid,
    pub pipeline_configuration_id: Uuid,
    pub document_version: u32,
    pub chunking_config: ChunkingConfig,
    pub request_id: Uuid,
    pub occurred_at: String,
}

pub struct CompleteChunking {
    pub chunk_set_id: Uuid,
    pub chunk_count: u32,
    pub occurred_at: String,
}

pub struct CompleteEmbedding {
    pub embedding_set_id: Uuid,
    pub occurred_at: String,
}

pub struct CompleteIndexing {
    pub vector_count: u32,
    pub occurred_at: String,
}

pub struct FailIngestion {
    pub stage: IngestStage,
    pub reason: String,
    pub occurred_at: String,
}

pub struct RetryIngestion {
    pub request_id: Uuid,
    pub occurred_at: String,
}

pub struct RemoveIndexing {
    pub occurred_at: String,
}

pub enum IndexingCommand {
    RequestIngest(RequestIngest),
    CompleteChunking(CompleteChunking),
    CompleteEmbedding(CompleteEmbedding),
    CompleteIndexing(CompleteIndexing),
    FailIngestion(FailIngestion),
    RetryIngestion(RetryIngestion),
    RemoveIndexing(RemoveIndexing),
}
