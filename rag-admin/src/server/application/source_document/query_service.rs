use std::sync::Arc;

use uuid::Uuid;

use crate::server::application::source_document::ports::ChunkSetRepository;
use crate::server::application::AppError;
use crate::server::domain::indexing::repository::IndexingRepository;
use crate::server::domain::source_document::repository::SourceDocumentRepository;
use crate::shared::{ChunkDto, IndexingDto, SourceDocumentDetailDto, SourceDocumentDto};

pub struct SourceDocumentQueryService {
    source_document_repository: Arc<dyn SourceDocumentRepository>,
    indexing_repository: Arc<dyn IndexingRepository>,
    chunk_set_repository: Arc<dyn ChunkSetRepository>,
}

impl SourceDocumentQueryService {
    pub fn new(
        source_document_repository: Arc<dyn SourceDocumentRepository>,
        indexing_repository: Arc<dyn IndexingRepository>,
        chunk_set_repository: Arc<dyn ChunkSetRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            source_document_repository,
            indexing_repository,
            chunk_set_repository,
        })
    }

    pub async fn list(&self) -> Result<Vec<SourceDocumentDto>, AppError> {
        let docs = self.source_document_repository.list().await?;
        Ok(docs.into_iter().map(map_doc_to_dto).collect())
    }

    pub async fn get_detail_by_source_ref(
        &self,
        source_ref: &crate::server::domain::source_document::source_ref::SourceRef,
    ) -> Result<Option<SourceDocumentDetailDto>, AppError> {
        let doc = self
            .source_document_repository
            .find_by_source_ref(source_ref)
            .await?;
        match doc {
            None => Ok(None),
            Some(doc) => self.get_detail(doc.document_id).await,
        }
    }

    pub async fn get_detail(
        &self,
        document_id: Uuid,
    ) -> Result<Option<SourceDocumentDetailDto>, AppError> {
        let doc = self.source_document_repository.load(document_id).await?;

        match doc {
            None => Ok(None),
            Some(doc) => {
                let indexings = self
                    .indexing_repository
                    .list_for_document(document_id)
                    .await?;

                Ok(Some(SourceDocumentDetailDto {
                    document: map_doc_to_dto(doc),
                    indexings: indexings
                        .into_iter()
                        .map(|i| IndexingDto {
                            indexing_id: i.indexing_id,
                            pipeline_configuration_id: i.pipeline_configuration_id,
                            document_version: i.document_version,
                            status: format!("{:?}", i.status),
                            attempts: i.attempts,
                            chunk_set_id: i.chunk_set_id,
                            embedding_set_id: i.embedding_set_id,
                            removed: i.removed,
                        })
                        .collect(),
                }))
            }
        }
    }

    pub async fn get_chunks(&self, chunk_set_id: Uuid) -> Result<Vec<ChunkDto>, AppError> {
        let chunks = self.chunk_set_repository.load_chunks(chunk_set_id).await?;
        Ok(chunks
            .into_iter()
            .map(|c| ChunkDto {
                chunk_id: c.chunk_id,
                sequence: c.sequence,
                heading: c.heading,
                text: c.text,
                char_start: c.char_start,
                char_end: c.char_end,
            })
            .collect())
    }
}

fn map_doc_to_dto(
    doc: crate::server::domain::source_document::read_model::SourceDocumentReadModel,
) -> SourceDocumentDto {
    use crate::server::domain::source_document::version::DocumentMetadata;
    let title = match &doc.latest_metadata {
        DocumentMetadata::BlogPost(m) => m.title.clone(),
    };
    SourceDocumentDto {
        document_id: doc.document_id,
        document_type: format!("{:?}", doc.document_type),
        source_ref_key: doc.source_ref.natural_key().to_string(),
        title,
        latest_version: doc.latest_version_number,
        latest_content_hash: doc.latest_content_hash.as_hex().to_string(),
        deleted: doc.deleted,
    }
}
