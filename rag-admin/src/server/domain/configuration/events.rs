pub use crate::server::domain::configuration::ai_provider::events::*;
pub use crate::server::domain::configuration::embedding_model::events::*;
pub use crate::server::domain::configuration::generation_model::events::*;
use crate::server::domain::configuration::pipeline_configuration::events::*;
pub use crate::server::domain::configuration::vector_index::events::*;
pub use crate::server::domain::configuration::vector_store_provider::events::*;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigurationCreated {
    pub configuration_id: Uuid,
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

    PipelineConfigurationCreated(PipelineConfigurationCreated),
    PipelineConfigurationUpdated(PipelineConfigurationUpdated),
    PipelineConfigurationDeleted(PipelineConfigurationDeleted),
}
