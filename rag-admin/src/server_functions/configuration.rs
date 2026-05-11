use leptos::prelude::*;

use crate::shared::{
    ChunkingConfigurationDto, ConfigurationCommandDto, ConfigurationDto, PipelineConfigurationDto,
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
    name = ApplyConfigurationCommand,
    prefix = "/api",
    endpoint = "apply_configuration_command"
)]
pub async fn apply_configuration_command(
    command: ConfigurationCommandDto,
) -> Result<(), ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .configuration_command_handler
        .handle_dto(command)
        .await
        .map_err(map_app_error)
}
