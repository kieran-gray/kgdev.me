use leptos::prelude::*;

use crate::shared::{
    ChunkingConfigurationCommandDto, ChunkingConfigurationDto, ConfigurationDto,
    EmbeddingModelCommandDto, GenerationModelCommandDto, PipelineConfigurationCommandDto,
    PipelineConfigurationDto, SweepTemplateCommandDto, SweepTemplateDto, VectorIndexCommandDto,
};

#[server(
    name = GetConfiguration,
    prefix = "/api",
    endpoint = "get_configuration"
)]
pub async fn get_configuration() -> Result<ConfigurationDto, ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .configuration_query_service
        .get()
        .await
        .map_err(map_app_error)
}

#[server(
    name = GetPipelineConfigurations,
    prefix = "/api",
    endpoint = "get_pipeline_configurations"
)]
pub async fn get_pipeline_configurations() -> Result<Vec<PipelineConfigurationDto>, ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .pipeline_configuration_query_service
        .list()
        .await
        .map_err(map_app_error)
}

#[server(
    name = GetChunkingConfigurations,
    prefix = "/api",
    endpoint = "get_chunking_configurations"
)]
pub async fn get_chunking_configurations() -> Result<Vec<ChunkingConfigurationDto>, ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .chunking_configuration_query_service
        .list()
        .await
        .map_err(map_app_error)
}

#[server(
    name = GetSweepTemplates,
    prefix = "/api",
    endpoint = "get_sweep_templates"
)]
pub async fn get_sweep_templates() -> Result<Vec<SweepTemplateDto>, ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .sweep_template_query_service
        .list()
        .await
        .map_err(map_app_error)
}

#[server(
    name = ApplyEmbeddingModelCommand,
    prefix = "/api",
    endpoint = "apply_embedding_model_command"
)]
pub async fn apply_embedding_model_command(
    command: EmbeddingModelCommandDto,
) -> Result<(), ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .embedding_model_command_handler
        .handle_dto(command)
        .await
        .map_err(map_app_error)
}

#[server(
    name = ApplyGenerationModelCommand,
    prefix = "/api",
    endpoint = "apply_generation_model_command"
)]
pub async fn apply_generation_model_command(
    command: GenerationModelCommandDto,
) -> Result<(), ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .generation_model_command_handler
        .handle_dto(command)
        .await
        .map_err(map_app_error)
}

#[server(
    name = ApplyVectorIndexCommand,
    prefix = "/api",
    endpoint = "apply_vector_index_command"
)]
pub async fn apply_vector_index_command(
    command: VectorIndexCommandDto,
) -> Result<(), ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .vector_index_command_handler
        .handle_dto(command)
        .await
        .map_err(map_app_error)
}

#[server(
    name = ApplyPipelineConfigurationCommand,
    prefix = "/api",
    endpoint = "apply_pipeline_configuration_command"
)]
pub async fn apply_pipeline_configuration_command(
    command: PipelineConfigurationCommandDto,
) -> Result<(), ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .pipeline_configuration_service
        .handle_dto(command)
        .await
        .map_err(map_app_error)
}

#[server(
    name = ApplyChunkingConfigurationCommand,
    prefix = "/api",
    endpoint = "apply_chunking_configuration_command"
)]
pub async fn apply_chunking_configuration_command(
    command: ChunkingConfigurationCommandDto,
) -> Result<(), ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .chunking_configuration_service
        .handle_dto(command)
        .await
        .map_err(map_app_error)
}

#[server(
    name = ApplySweepTemplateCommand,
    prefix = "/api",
    endpoint = "apply_sweep_template_command"
)]
pub async fn apply_sweep_template_command(
    command: SweepTemplateCommandDto,
) -> Result<(), ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .sweep_template_command_handler
        .handle_dto(command)
        .await
        .map_err(map_app_error)
}
