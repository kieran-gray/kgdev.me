use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    server::domain::configuration::{
        embedding_model::EmbeddingModel, generation_model::GenerationModel,
        vector_index::VectorIndex,
    },
    shared::{
        AiProviderKindDto, ConfigurationDto, EmbeddingModelDto, GenerationModelDto, VectorIndexDto,
        VectorStoreKindDto,
    },
};

use super::aggregate::Configuration;
use super::kinds::{AiProviderKind, VectorStoreKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationReadModel {
    pub configuration_id: Uuid,
    pub embedding_models: Vec<EmbeddingModel>,
    pub generation_models: Vec<GenerationModel>,
    pub vector_indexes: Vec<VectorIndex>,
}

impl Default for ConfigurationReadModel {
    fn default() -> Self {
        Self {
            configuration_id: Uuid::nil(),
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
            embedding_models: c.embedding_models.clone(),
            generation_models: c.generation_models.clone(),
            vector_indexes: c.vector_indexes.clone(),
        }
    }
}

impl From<&ConfigurationReadModel> for ConfigurationDto {
    fn from(value: &ConfigurationReadModel) -> Self {
        Self {
            configuration_id: value.configuration_id,
            embedding_models: value
                .embedding_models
                .iter()
                .map(|m| EmbeddingModelDto {
                    embedding_model_id: m.embedding_model_id,
                    kind: ai_provider_kind_dto(m.kind),
                    model: m.model.clone(),
                    dimensions: m.dimensions,
                })
                .collect(),
            generation_models: value
                .generation_models
                .iter()
                .map(|m| GenerationModelDto {
                    generation_model_id: m.generation_model_id,
                    kind: ai_provider_kind_dto(m.kind),
                    model: m.model.clone(),
                })
                .collect(),
            vector_indexes: value
                .vector_indexes
                .iter()
                .map(|i| VectorIndexDto {
                    index_id: i.index_id,
                    kind: vector_store_kind_dto(i.kind),
                    name: i.name.clone(),
                    dimensions: i.dimensions,
                })
                .collect(),
        }
    }
}

pub(crate) fn ai_provider_kind_dto(kind: AiProviderKind) -> AiProviderKindDto {
    match kind {
        AiProviderKind::Cloudflare => AiProviderKindDto::Cloudflare,
        AiProviderKind::Ollama => AiProviderKindDto::Ollama,
    }
}

pub(crate) fn vector_store_kind_dto(kind: VectorStoreKind) -> VectorStoreKindDto {
    match kind {
        VectorStoreKind::CloudflareVectorize => VectorStoreKindDto::CloudflareVectorize,
    }
}
