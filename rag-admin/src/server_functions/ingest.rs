use leptos::prelude::*;

use crate::shared::{IngestJobInfo, IngestOptions};

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
