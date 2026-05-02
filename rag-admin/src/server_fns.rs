use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::shared::{
    EmbedResult, IngestJobInfo, IngestOptions, PostDetailDto, PostSummary, SettingsDto,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
}

#[server(name = ListPosts, prefix = "/api", endpoint = "list_posts")]
pub async fn list_posts() -> Result<Vec<PostSummary>, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .ingest_service
        .list_posts()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(name = GetPostDetail, prefix = "/api", endpoint = "get_post_detail")]
pub async fn get_post_detail(slug: String) -> Result<PostDetailDto, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .ingest_service
        .get_post_detail(&slug)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(name = StartIngest, prefix = "/api", endpoint = "start_ingest")]
pub async fn start_ingest(
    slug: String,
    options: IngestOptions,
) -> Result<IngestJobInfo, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .ingest_service
        .start_ingest(slug, options)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(name = EmbedTexts, prefix = "/api", endpoint = "embed_texts")]
pub async fn embed_texts(
    model: String,
    text_a: String,
    text_b: String,
) -> Result<EmbedResult, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .ingest_service
        .embed_texts(&model, &text_a, &text_b)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

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
