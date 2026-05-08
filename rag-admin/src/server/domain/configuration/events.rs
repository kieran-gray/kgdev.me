pub use crate::server::domain::ai_provider::events::*;
pub use crate::server::domain::embedding_model::events::*;
pub use crate::server::domain::generation_model::events::*;
pub use crate::server::domain::vector_index::events::*;
pub use crate::server::domain::vector_store_provider::events::*;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigurationCreated {
    pub configuration_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurrentEmbeddingModelSet {
    pub model_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurrentGenerationModelSet {
    pub model_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurrentVectorIndexSet {
    pub index_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum ConfigurationEvent {
    ConfigurationCreated(ConfigurationCreated),
    AiProviderAdded(AiProviderAdded),
    AiProviderUpdated(AiProviderUpdated),
    AiProviderRemoved(AiProviderRemoved),

    EmbeddingModelAdded(EmbeddingModelAdded),
    EmbeddingModelUpdated(EmbeddingModelUpdated),
    EmbeddingModelRemoved(EmbeddingModelRemoved),

    GenerationModelAdded(GenerationModelAdded),
    GenerationModelUpdated(GenerationModelUpdated),
    GenerationModelRemoved(GenerationModelRemoved),

    VectorStoreProviderAdded(VectorStoreProviderAdded),
    VectorStoreProviderUpdated(VectorStoreProviderUpdated),
    VectorStoreProviderRemoved(VectorStoreProviderRemoved),

    VectorIndexAdded(VectorIndexAdded),
    VectorIndexUpdated(VectorIndexUpdated),
    VectorIndexRemoved(VectorIndexRemoved),

    CurrentEmbeddingModelSet(CurrentEmbeddingModelSet),
    CurrentGenerationModelSet(CurrentGenerationModelSet),
    CurrentVectorIndexSet(CurrentVectorIndexSet),
}
