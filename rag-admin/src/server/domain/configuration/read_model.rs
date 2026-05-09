use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::domain::{
    ai_provider::entity::AiProvdier, embedding_model::entity::EmbeddingModel,
    generation_model::entity::GenerationModel, vector_index::entity::VectorIndex,
    vector_store_provider::entity::VectorStoreProvider,
};

use super::aggregate::Configuration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationReadModel {
    pub configuration_id: Uuid,
    pub ai_providers: Vec<AiProvdier>,
    pub vector_store_providers: Vec<VectorStoreProvider>,
    pub embedding_models: Vec<EmbeddingModel>,
    pub generation_models: Vec<GenerationModel>,
    pub vector_indexes: Vec<VectorIndex>,
}

impl Default for ConfigurationReadModel {
    fn default() -> Self {
        Self {
            configuration_id: Uuid::nil(),
            ai_providers: Vec::new(),
            vector_store_providers: Vec::new(),
            embedding_models: Vec::new(),
            generation_models: Vec::new(),
            vector_indexes: Vec::new(),
        }
    }
}

impl From<&Configuration> for ConfigurationReadModel {
    fn from(c: &Configuration) -> Self {
        Self {
            configuration_id: c.configuration_id,
            ai_providers: c.ai_providers.clone(),
            vector_store_providers: c.vector_store_providers.clone(),
            embedding_models: c.embedding_models.clone(),
            generation_models: c.generation_models.clone(),
            vector_indexes: c.vector_indexes.clone(),
        }
    }
}
