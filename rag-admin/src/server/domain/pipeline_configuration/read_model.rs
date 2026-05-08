use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::domain::{
    ai_provider::entity::AiProvdier, embedding_model::entity::EmbeddingModel,
    generation_model::entity::GenerationModel, vector_index::entity::VectorIndex,
    vector_store_provider::entity::VectorStoreProvider,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfiguration {
    pub configuration_id: Uuid,
    pub ai_providers: Vec<AiProvdier>,
    pub vector_store_providers: Vec<VectorStoreProvider>,
    pub embedding_models: Vec<EmbeddingModel>,
    pub generation_models: Vec<GenerationModel>,
    pub vector_indexes: Vec<VectorIndex>,
    pub current_embedding_model_id: Option<Uuid>,
    pub current_generation_model_id: Option<Uuid>,
    pub current_vector_index_id: Option<Uuid>,
}

impl Default for PipelineConfiguration {
    fn default() -> Self {
        Self {
            configuration_id: Uuid::nil(),
            ai_providers: Vec::new(),
            vector_store_providers: Vec::new(),
            embedding_models: Vec::new(),
            generation_models: Vec::new(),
            vector_indexes: Vec::new(),
            current_embedding_model_id: None,
            current_generation_model_id: None,
            current_vector_index_id: None,
        }
    }
}

impl PipelineConfiguration {
    pub fn current_embedding_model(&self) -> Option<&EmbeddingModel> {
        self.current_embedding_model_id.and_then(|model_id| {
            self.embedding_models
                .iter()
                .find(|model| model.embedding_model_id == model_id)
        })
    }

    pub fn current_generation_model(&self) -> Option<&GenerationModel> {
        self.current_generation_model_id.and_then(|model_id| {
            self.generation_models
                .iter()
                .find(|model| model.generation_model_id == model_id)
        })
    }

    pub fn current_vector_index(&self) -> Option<&VectorIndex> {
        self.current_vector_index_id.and_then(|index_id| {
            self.vector_indexes
                .iter()
                .find(|index| index.index_id == index_id)
        })
    }

    pub fn provider(&self, provider_id: Uuid) -> Option<&AiProvdier> {
        self.ai_providers
            .iter()
            .find(|provider| provider.provider_id == provider_id)
    }

    pub fn current_embedding_provider(&self) -> Option<&AiProvdier> {
        self.current_embedding_model()
            .and_then(|model| self.provider(model.provider_id))
    }

    pub fn current_generation_provider(&self) -> Option<&AiProvdier> {
        self.current_generation_model()
            .and_then(|model| self.provider(model.provider_id))
    }
}
