use leptos::prelude::*;

use crate::shared::ChunkingConfig;

#[server(
    name = SavePostChunkingConfig,
    prefix = "/api",
    endpoint = "save_post_chunking_config"
)]
pub async fn save_post_chunking_config(
    slug: String,
    config: ChunkingConfig,
) -> Result<(), ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .post_chunking_config_store
        .save(&slug, config)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(
    name = ClearPostChunkingConfig,
    prefix = "/api",
    endpoint = "clear_post_chunking_config"
)]
pub async fn clear_post_chunking_config(slug: String) -> Result<(), ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .post_chunking_config_store
        .clear(&slug)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
