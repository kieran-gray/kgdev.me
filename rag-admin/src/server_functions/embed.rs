use leptos::prelude::*;

use crate::shared::EmbedResult;

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
        .embedding_service
        .embed_texts(&model, &text_a, &text_b)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
