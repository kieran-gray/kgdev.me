pub use crate::server::domain::configuration::embedding_model::commands::*;
pub use crate::server::domain::configuration::generation_model::commands::*;
pub use crate::server::domain::configuration::vector_index::commands::*;

use crate::{
    server::domain::configuration::{
        chunking_configuration::{
            CreateChunkingConfiguration, DeleteChunkingConfiguration, UpdateChunkingConfiguration,
        },
        kinds::{AiProviderKind, VectorStoreKind},
        pipeline_configuration::{
            CreatePipelineConfiguration, DeletePipelineConfiguration, UpdatePipelineConfiguration,
        },
    },
    shared::{AiProviderKindDto, ConfigurationCommandDto, VectorStoreKindDto},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ConfigurationCommand {
    AddEmbeddingModel(AddEmbeddingModel),
    UpdateEmbeddingModel(UpdateEmbeddingModel),
    RemoveEmbeddingModel(RemoveEmbeddingModel),

    AddGenerationModel(AddGenerationModel),
    UpdateGenerationModel(UpdateGenerationModel),
    RemoveGenerationModel(RemoveGenerationModel),

    AddVectorIndex(AddVectorIndex),
    UpdateVectorIndex(UpdateVectorIndex),
    RemoveVectorIndex(RemoveVectorIndex),

    CreatePipelineConfiguration(CreatePipelineConfiguration),
    UpdatePipelineConfiguration(UpdatePipelineConfiguration),
    DeletePipelineConfiguration(DeletePipelineConfiguration),

    CreateChunkingConfiguration(CreateChunkingConfiguration),
    UpdateChunkingConfiguration(UpdateChunkingConfiguration),
    DeleteChunkingConfiguration(DeleteChunkingConfiguration),
}

impl From<ConfigurationCommandDto> for ConfigurationCommand {
    fn from(value: ConfigurationCommandDto) -> Self {
        match value {
            ConfigurationCommandDto::AddEmbeddingModel(dto) => {
                ConfigurationCommand::AddEmbeddingModel(AddEmbeddingModel {
                    kind: ai_kind(dto.kind),
                    model: dto.model,
                    dimensions: dto.dimensions,
                })
            }
            ConfigurationCommandDto::UpdateEmbeddingModel(dto) => {
                ConfigurationCommand::UpdateEmbeddingModel(UpdateEmbeddingModel {
                    model_id: dto.model_id,
                    kind: ai_kind(dto.kind),
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
                    kind: ai_kind(dto.kind),
                    model: dto.model,
                })
            }
            ConfigurationCommandDto::UpdateGenerationModel(dto) => {
                ConfigurationCommand::UpdateGenerationModel(UpdateGenerationModel {
                    model_id: dto.model_id,
                    kind: ai_kind(dto.kind),
                    model: dto.model,
                })
            }
            ConfigurationCommandDto::RemoveGenerationModel(dto) => {
                ConfigurationCommand::RemoveGenerationModel(RemoveGenerationModel {
                    model_id: dto.model_id,
                })
            }
            ConfigurationCommandDto::AddVectorIndex(dto) => {
                ConfigurationCommand::AddVectorIndex(AddVectorIndex {
                    kind: vector_kind(dto.kind),
                    name: dto.name,
                    dimensions: dto.dimensions,
                })
            }
            ConfigurationCommandDto::UpdateVectorIndex(dto) => {
                ConfigurationCommand::UpdateVectorIndex(UpdateVectorIndex {
                    index_id: dto.index_id,
                    kind: vector_kind(dto.kind),
                    name: dto.name,
                    dimensions: dto.dimensions,
                })
            }
            ConfigurationCommandDto::RemoveVectorIndex(dto) => {
                ConfigurationCommand::RemoveVectorIndex(RemoveVectorIndex {
                    index_id: dto.index_id,
                })
            }
            ConfigurationCommandDto::CreatePipelineConfiguration(dto) => {
                ConfigurationCommand::CreatePipelineConfiguration(CreatePipelineConfiguration {
                    name: dto.name,
                    embedding_model_id: dto.embedding_model_id,
                    generation_model_id: dto.generation_model_id,
                    vector_index_id: dto.vector_index_id,
                })
            }
            ConfigurationCommandDto::UpdatePipelineConfiguration(dto) => {
                ConfigurationCommand::UpdatePipelineConfiguration(UpdatePipelineConfiguration {
                    pipeline_configuration_id: dto.pipeline_configuration_id,
                    name: dto.name,
                    embedding_model_id: dto.embedding_model_id,
                    generation_model_id: dto.generation_model_id,
                    vector_index_id: dto.vector_index_id,
                })
            }
            ConfigurationCommandDto::DeletePipelineConfiguration(dto) => {
                ConfigurationCommand::DeletePipelineConfiguration(DeletePipelineConfiguration {
                    pipeline_configuration_id: dto.pipeline_configuration_id,
                })
            }
            ConfigurationCommandDto::CreateChunkingConfiguration(dto) => {
                ConfigurationCommand::CreateChunkingConfiguration(CreateChunkingConfiguration {
                    name: dto.name,
                    config: dto.config,
                })
            }
            ConfigurationCommandDto::UpdateChunkingConfiguration(dto) => {
                ConfigurationCommand::UpdateChunkingConfiguration(UpdateChunkingConfiguration {
                    chunking_configuration_id: dto.chunking_configuration_id,
                    name: dto.name,
                    config: dto.config,
                })
            }
            ConfigurationCommandDto::DeleteChunkingConfiguration(dto) => {
                ConfigurationCommand::DeleteChunkingConfiguration(DeleteChunkingConfiguration {
                    chunking_configuration_id: dto.chunking_configuration_id,
                })
            }
        }
    }
}

fn ai_kind(dto: AiProviderKindDto) -> AiProviderKind {
    match dto {
        AiProviderKindDto::Cloudflare => AiProviderKind::Cloudflare,
        AiProviderKindDto::Ollama => AiProviderKind::Ollama,
    }
}

fn vector_kind(dto: VectorStoreKindDto) -> VectorStoreKind {
    match dto {
        VectorStoreKindDto::CloudflareVectorize => VectorStoreKind::CloudflareVectorize,
    }
}
