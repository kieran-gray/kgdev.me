use leptos::prelude::*;

use crate::shared::{ChunkingConfig, IngestJobInfo, SourceDocumentDetailDto, SourceDocumentDto};

#[server(
    name = StartSourceDocumentIngest,
    prefix = "/api",
    endpoint = "start_source_document_ingest"
)]
pub async fn start_source_document_ingest(
    source_ref_slug: String,
    pipeline_configuration_id: uuid::Uuid,
    chunking_config: ChunkingConfig,
) -> Result<IngestJobInfo, ServerFnError> {
    use crate::server::domain::source_document::document_type::DocumentType;
    use crate::server::domain::source_document::source_ref::SourceRef;
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .source_document_ingest_service
        .start_ingest(
            SourceRef::UpstreamSlug {
                slug: source_ref_slug,
            },
            DocumentType::BlogPost,
            pipeline_configuration_id,
            chunking_config,
        )
        .await
        .map_err(map_app_error)
}

#[server(
    name = ListSourceDocuments,
    prefix = "/api",
    endpoint = "list_source_documents"
)]
pub async fn list_source_documents() -> Result<Vec<SourceDocumentDto>, ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .source_document_query_service
        .list()
        .await
        .map_err(map_app_error)
}

#[server(
    name = GetSourceDocumentDetail,
    prefix = "/api",
    endpoint = "get_source_document_detail"
)]
pub async fn get_source_document_detail(
    document_id: uuid::Uuid,
) -> Result<Option<SourceDocumentDetailDto>, ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .source_document_query_service
        .get_detail(document_id)
        .await
        .map_err(map_app_error)
}
