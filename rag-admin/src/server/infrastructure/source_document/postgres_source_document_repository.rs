use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::server::domain::source_document::{
    read_model::SourceDocumentReadModel,
    repository::{SourceDocumentRepository, SourceDocumentRepositoryError},
    source_ref::SourceRef,
};

pub struct PostgresSourceDocumentRepository {
    pool: PgPool,
}

impl PostgresSourceDocumentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SourceDocumentRepository for PostgresSourceDocumentRepository {
    async fn load(
        &self,
        document_id: Uuid,
    ) -> Result<Option<SourceDocumentReadModel>, SourceDocumentRepositoryError> {
        let row: Option<(serde_json::Value,)> =
            sqlx::query_as("SELECT read_model FROM source_documents WHERE document_id = $1")
                .bind(document_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| SourceDocumentRepositoryError::Internal(format!("load: {e}")))?;

        match row {
            None => Ok(None),
            Some((json,)) => serde_json::from_value(json)
                .map(Some)
                .map_err(|e| SourceDocumentRepositoryError::Internal(format!("deserialize: {e}"))),
        }
    }

    async fn save(
        &self,
        read_model: SourceDocumentReadModel,
    ) -> Result<(), SourceDocumentRepositoryError> {
        let json = serde_json::to_value(&read_model)
            .map_err(|e| SourceDocumentRepositoryError::Internal(format!("serialize: {e}")))?;

        sqlx::query(
            r#"
            INSERT INTO source_documents (document_id, read_model, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (document_id) DO UPDATE SET
                read_model = $2,
                updated_at = NOW()
            "#,
        )
        .bind(read_model.document_id)
        .bind(&json)
        .execute(&self.pool)
        .await
        .map_err(|e| SourceDocumentRepositoryError::Internal(format!("save: {e}")))?;

        Ok(())
    }

    async fn list(&self) -> Result<Vec<SourceDocumentReadModel>, SourceDocumentRepositoryError> {
        let rows: Vec<(serde_json::Value,)> =
            sqlx::query_as("SELECT read_model FROM source_documents ORDER BY updated_at DESC")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| SourceDocumentRepositoryError::Internal(format!("list: {e}")))?;

        rows.into_iter()
            .map(|(json,)| {
                serde_json::from_value(json).map_err(|e| {
                    SourceDocumentRepositoryError::Internal(format!("deserialize: {e}"))
                })
            })
            .collect()
    }

    async fn find_by_source_ref(
        &self,
        source_ref: &SourceRef,
    ) -> Result<Option<SourceDocumentReadModel>, SourceDocumentRepositoryError> {
        let source_ref_json = serde_json::to_value(source_ref).map_err(|e| {
            SourceDocumentRepositoryError::Internal(format!("serialize source_ref: {e}"))
        })?;

        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT read_model FROM source_documents WHERE read_model->'source_ref' = $1 LIMIT 1",
        )
        .bind(&source_ref_json)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SourceDocumentRepositoryError::Internal(format!("find_by_source_ref: {e}")))?;

        match row {
            None => Ok(None),
            Some((json,)) => serde_json::from_value(json)
                .map(Some)
                .map_err(|e| SourceDocumentRepositoryError::Internal(format!("deserialize: {e}"))),
        }
    }
}
