use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDocumentDto {
    pub document_id: Uuid,
    pub document_type: String,
    pub source_ref_key: String,
    pub title: String,
    pub latest_version: u32,
    pub latest_content_hash: String,
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingDto {
    pub indexing_id: Uuid,
    pub pipeline_configuration_id: Uuid,
    pub document_version: u32,
    pub status: String,
    pub attempts: u32,
    pub chunk_set_id: Option<Uuid>,
    pub embedding_set_id: Option<Uuid>,
    pub removed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDocumentDetailDto {
    pub document: SourceDocumentDto,
    pub indexings: Vec<IndexingDto>,
}
