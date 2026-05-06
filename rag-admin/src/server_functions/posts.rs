use leptos::prelude::*;

use crate::shared::{ChunkingConfig, PostDetailDto, PostSummary};

#[server(name = ListPosts, prefix = "/api", endpoint = "list_posts")]
pub async fn list_posts() -> Result<Vec<PostSummary>, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .post_service
        .list_posts()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(name = GetPostDetail, prefix = "/api", endpoint = "get_post_detail")]
pub async fn get_post_detail(
    slug: String,
    chunking_override: Option<ChunkingConfig>,
) -> Result<PostDetailDto, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .post_service
        .get_post_detail(&slug, chunking_override)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
