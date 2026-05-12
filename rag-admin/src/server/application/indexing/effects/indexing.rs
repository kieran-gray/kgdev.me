use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Run the chunking stage for an indexing aggregate. Emitted by the policy
/// on `IngestRequested` or `ChunkingRequeued`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteChunkingEffect {
    pub indexing_id: Uuid,
}

/// Run the embedding stage. Emitted on `EmbeddingRequeued` or on
/// `ChunkingCompleted` when the aggregate's `auto_advance` is true.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteEmbeddingEffect {
    pub indexing_id: Uuid,
}

/// Run the upsert-to-vector-index stage. Emitted on `IndexingRequeued` or on
/// `EmbeddingCompleted` when `auto_advance` is true.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteIndexingEffect {
    pub indexing_id: Uuid,
}

/// All side-effecting work the indexing process manager dispatches.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum IndexingEffect {
    ExecuteChunking(ExecuteChunkingEffect),
    ExecuteEmbedding(ExecuteEmbeddingEffect),
    ExecuteIndexing(ExecuteIndexingEffect),
}
