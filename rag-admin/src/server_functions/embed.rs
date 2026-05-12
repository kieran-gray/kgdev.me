use leptos::prelude::*;

use crate::shared::EmbedResult;

#[server(name = EmbedTexts, prefix = "/api", endpoint = "embed_texts")]
pub async fn embed_texts(
    model: String,
    text_a: String,
    text_b: String,
) -> Result<EmbedResult, ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    let configuration = state
        .configuration_query_service
        .get()
        .await
        .map_err(map_app_error)?;
    let embedding_model_id = configuration
        .embedding_models
        .iter()
        .find(|m| m.model == model)
        .map(|m| m.embedding_model_id)
        .ok_or_else(|| {
            ServerFnError::new(format!(
                "embedding model '{model}' is not registered in the configuration"
            ))
        })?;

    state
        .embedding_service
        .embed_texts(embedding_model_id, &text_a, &text_b)
        .await
        .map_err(map_app_error)
}
