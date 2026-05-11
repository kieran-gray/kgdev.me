use crate::server::domain::configuration::chunking_configuration::events::*;
pub use crate::server::domain::configuration::embedding_model::events::*;
pub use crate::server::domain::configuration::generation_model::events::*;
use crate::server::domain::configuration::pipeline_configuration::events::*;
pub use crate::server::domain::configuration::vector_index::events::*;

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

    EmbeddingModelAdded(EmbeddingModelAdded),
    EmbeddingModelUpdated(EmbeddingModelUpdated),
    EmbeddingModelRemoved(EmbeddingModelRemoved),

    GenerationModelAdded(GenerationModelAdded),
    GenerationModelUpdated(GenerationModelUpdated),
    GenerationModelRemoved(GenerationModelRemoved),

    VectorIndexAdded(VectorIndexAdded),
    VectorIndexUpdated(VectorIndexUpdated),
    VectorIndexRemoved(VectorIndexRemoved),

    PipelineConfigurationCreated(PipelineConfigurationCreated),
    PipelineConfigurationUpdated(PipelineConfigurationUpdated),
    PipelineConfigurationDeleted(PipelineConfigurationDeleted),

    ChunkingConfigurationCreated(ChunkingConfigurationCreated),
    ChunkingConfigurationUpdated(ChunkingConfigurationUpdated),
    ChunkingConfigurationDeleted(ChunkingConfigurationDeleted),
}
