use leptos::prelude::*;

use crate::shared::SettingsDto;

#[server(name = LoadSettings, prefix = "/api", endpoint = "load_settings")]
pub async fn load_settings() -> Result<SettingsDto, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    Ok(state.settings_snapshot().await)
}

#[server(name = SaveSettings, prefix = "/api", endpoint = "save_settings")]
pub async fn save_settings(settings: SettingsDto) -> Result<(), ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .save_settings(settings)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
