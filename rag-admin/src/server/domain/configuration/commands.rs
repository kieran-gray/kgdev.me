pub use crate::server::domain::ai_provider::commands::*;
pub use crate::server::domain::embedding_model::commands::*;
pub use crate::server::domain::generation_model::commands::*;
pub use crate::server::domain::vector_index::commands::*;
pub use crate::server::domain::vector_store_provider::commands::*;

use crate::shared::{ConfigurationCommandDto, ProviderType};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetCurrentEmbeddingModel {
    pub model_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetCurrentGenerationModel {
    pub model_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetCurrentVectorIndex {
    pub index_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ConfigurationCommand {
    AddAiProvider(AddAiProvider),
    UpdateAiProvider(UpdateAiProvider),
    RemoveAiProvider(RemoveAiProvider),

    AddEmbeddingModel(AddEmbeddingModel),
    UpdateEmbeddingModel(UpdateEmbeddingModel),
    RemoveEmbeddingModel(RemoveEmbeddingModel),

    AddGenerationModel(AddGenerationModel),
    UpdateGenerationModel(UpdateGenerationModel),
    RemoveGenerationModel(RemoveGenerationModel),

    AddVectorStoreProvider(AddVectorStoreProvider),
    UpdateVectorStoreProvider(UpdateVectorStoreProvider),
    RemoveVectorStoreProvider(RemoveVectorStoreProvider),

    AddVectorIndex(AddVectorIndex),
    UpdateVectorIndex(UpdateVectorIndex),
    RemoveVectorIndex(RemoveVectorIndex),

    SetCurrentEmbeddingModel(SetCurrentEmbeddingModel),
    SetCurrentGenerationModel(SetCurrentGenerationModel),
    SetCurrentVectorIndex(SetCurrentVectorIndex),
}

impl From<ConfigurationCommandDto> for ConfigurationCommand {
    fn from(value: ConfigurationCommandDto) -> Self {
        match value {
            ConfigurationCommandDto::AddProvider(dto) => match dto.provider_type {
                ProviderType::Ai => {
                    ConfigurationCommand::AddAiProvider(AddAiProvider { name: dto.name })
                }
                ProviderType::VectorStore => ConfigurationCommand::AddVectorStoreProvider(
                    AddVectorStoreProvider { name: dto.name },
                ),
            },
            ConfigurationCommandDto::UpdateAiProvider(dto) => {
                ConfigurationCommand::UpdateAiProvider(UpdateAiProvider {
                    provider_id: dto.provider_id,
                    name: dto.name,
                })
            }
            ConfigurationCommandDto::RemoveAiProvider(dto) => {
                ConfigurationCommand::RemoveAiProvider(RemoveAiProvider {
                    provider_id: dto.provider_id,
                })
            }
            ConfigurationCommandDto::AddEmbeddingModel(dto) => {
                ConfigurationCommand::AddEmbeddingModel(AddEmbeddingModel {
                    provider_id: dto.provider_id,
                    model: dto.model,
                    dimensions: dto.dimensions,
                })
            }
            ConfigurationCommandDto::UpdateEmbeddingModel(dto) => {
                ConfigurationCommand::UpdateEmbeddingModel(UpdateEmbeddingModel {
                    model_id: dto.model_id,
                    provider_id: dto.provider_id,
                    model: dto.model,
                    dimensions: dto.dimensions,
                })
            }
            ConfigurationCommandDto::RemoveEmbeddingModel(dto) => {
                ConfigurationCommand::RemoveEmbeddingModel(RemoveEmbeddingModel {
                    model_id: dto.model_id,
                })
            }
            ConfigurationCommandDto::AddGenerationModel(dto) => {
                ConfigurationCommand::AddGenerationModel(AddGenerationModel {
                    provider_id: dto.provider_id,
                    model: dto.model,
                })
            }
            ConfigurationCommandDto::UpdateGenerationModel(dto) => {
                ConfigurationCommand::UpdateGenerationModel(UpdateGenerationModel {
                    model_id: dto.model_id,
                    provider_id: dto.provider_id,
                    model: dto.model,
                })
            }
            ConfigurationCommandDto::RemoveGenerationModel(dto) => {
                ConfigurationCommand::RemoveGenerationModel(RemoveGenerationModel {
                    model_id: dto.model_id,
                })
            }
            ConfigurationCommandDto::UpdateVectorStoreProvider(dto) => {
                ConfigurationCommand::UpdateVectorStoreProvider(UpdateVectorStoreProvider {
                    provider_id: dto.provider_id,
                    name: dto.name,
                })
            }
            ConfigurationCommandDto::RemoveVectorStoreProvider(dto) => {
                ConfigurationCommand::RemoveVectorStoreProvider(RemoveVectorStoreProvider {
                    provider_id: dto.provider_id,
                })
            }
            ConfigurationCommandDto::AddVectorIndex(dto) => {
                ConfigurationCommand::AddVectorIndex(AddVectorIndex {
                    vector_store_provider_id: dto.vector_store_provider_id,
                    name: dto.name,
                    dimensions: dto.dimensions,
                })
            }
            ConfigurationCommandDto::UpdateVectorIndex(dto) => {
                ConfigurationCommand::UpdateVectorIndex(UpdateVectorIndex {
                    index_id: dto.index_id,
                    vector_store_provider_id: dto.vector_store_provider_id,
                    name: dto.name,
                    dimensions: dto.dimensions,
                })
            }
            ConfigurationCommandDto::RemoveVectorIndex(dto) => {
                ConfigurationCommand::RemoveVectorIndex(RemoveVectorIndex {
                    index_id: dto.index_id,
                })
            }
            ConfigurationCommandDto::SetCurrentEmbeddingModel(dto) => {
                ConfigurationCommand::SetCurrentEmbeddingModel(SetCurrentEmbeddingModel {
                    model_id: dto.model_id,
                })
            }
            ConfigurationCommandDto::SetCurrentGenerationModel(dto) => {
                ConfigurationCommand::SetCurrentGenerationModel(SetCurrentGenerationModel {
                    model_id: dto.model_id,
                })
            }
            ConfigurationCommandDto::SetCurrentVectorIndex(dto) => {
                ConfigurationCommand::SetCurrentVectorIndex(SetCurrentVectorIndex {
                    index_id: dto.index_id,
                })
            }
        }
    }
}
