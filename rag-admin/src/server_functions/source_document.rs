use leptos::prelude::*;

use crate::shared::{
    ChunkingConfig, DocumentListItemDto, IngestJobInfo, SourceDocumentDetailDto, SourceDocumentDto,
};

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
    name = ListDocumentsWithStatus,
    prefix = "/api",
    endpoint = "list_documents_with_status"
)]
pub async fn list_documents_with_status() -> Result<Vec<DocumentListItemDto>, ServerFnError> {
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    let available = state
        .source_adapter_registry
        .list_all()
        .await
        .map_err(map_app_error)?;

    let existing: Vec<SourceDocumentDto> = state
        .source_document_query_service
        .list()
        .await
        .map_err(map_app_error)?;

    let mut existing_map: std::collections::HashMap<String, SourceDocumentDto> = existing
        .into_iter()
        .map(|d| (d.source_ref_key.clone(), d))
        .collect();

    let mut items: Vec<DocumentListItemDto> = available
        .into_iter()
        .map(|(doc_type, summary)| {
            let key = summary.source_ref.natural_key().to_string();
            match existing_map.remove(&key) {
                Some(existing_doc) => DocumentListItemDto {
                    source_ref_key: key,
                    document_type: doc_type,
                    title: summary.title,
                    document_id: Some(existing_doc.document_id),
                    latest_version: Some(existing_doc.latest_version),
                    latest_content_hash: Some(existing_doc.latest_content_hash),
                    indexings: vec![],
                },
                None => DocumentListItemDto {
                    source_ref_key: key,
                    document_type: doc_type,
                    title: summary.title,
                    document_id: None,
                    latest_version: None,
                    latest_content_hash: None,
                    indexings: vec![],
                },
            }
        })
        .collect();

    for leftover in existing_map.into_values() {
        items.push(DocumentListItemDto {
            source_ref_key: leftover.source_ref_key.clone(),
            document_type: leftover.document_type,
            title: leftover.title,
            document_id: Some(leftover.document_id),
            latest_version: Some(leftover.latest_version),
            latest_content_hash: Some(leftover.latest_content_hash),
            indexings: vec![],
        });
    }

    Ok(items)
}

#[server(
    name = GetDocumentDetailBySourceRef,
    prefix = "/api",
    endpoint = "get_document_detail_by_source_ref"
)]
pub async fn get_document_detail_by_source_ref(
    source_ref_slug: String,
) -> Result<Option<SourceDocumentDetailDto>, ServerFnError> {
    use crate::server::domain::source_document::source_ref::SourceRef;
    use crate::server::setup::AppState;
    use crate::server_functions::error::map_app_error;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .source_document_query_service
        .get_detail_by_source_ref(&SourceRef::UpstreamSlug {
            slug: source_ref_slug,
        })
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
