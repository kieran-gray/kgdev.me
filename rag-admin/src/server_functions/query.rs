use leptos::prelude::*;

use crate::shared::{QueryRequest, QueryResult};

#[server(name = QueryDocuments, prefix = "/api", endpoint = "query_documents")]
pub async fn query_documents(req: QueryRequest) -> Result<QueryResult, ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state.query_service.query(req).await.map_err(map_app_error)
}
