use async_trait::async_trait;

use crate::server::application::AppError;
use crate::server::domain::source_document::{
    document_type::DocumentType, source_ref::SourceRef, version::DocumentMetadata,
};

pub struct DocumentSummary {
    pub source_ref: SourceRef,
    pub title: String,
}

/// Raw document content fetched from a source.
pub struct FetchedDocument {
    /// Natural identifier in source terms.
    pub source_ref: SourceRef,
    /// Raw bytes to be stored in the blob store. Chunking decodes these as
    /// UTF-8 markdown — adapters are responsible for normalising their source
    /// format into markdown bytes here.
    pub content: Vec<u8>,
    /// Typed metadata (title, published_at, etc.).
    pub metadata: DocumentMetadata,
}

#[async_trait]
pub trait SourceAdapter: Send + Sync {
    fn document_type(&self) -> DocumentType;
    async fn list(&self) -> Result<Vec<DocumentSummary>, AppError>;
    async fn fetch(&self, source_ref: &SourceRef) -> Result<FetchedDocument, AppError>;
}
