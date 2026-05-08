use leptos::prelude::*;

use crate::shared::{ConfigurationCommandDto, PipelineConfigurationDto};

#[server(
    name = GetPipelineConfiguration,
    prefix = "/api",
    endpoint = "get_pipeline_configuration"
)]
pub async fn get_pipeline_configuration() -> Result<PipelineConfigurationDto, ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .pipeline_configuration_service
        .get()
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
