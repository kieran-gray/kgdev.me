use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::server::domain::indexing::{
    read_model::IndexingReadModel,
    repository::{IndexingRepository, IndexingRepositoryError},
};

pub struct PostgresIndexingRepository {
    pool: PgPool,
}

impl PostgresIndexingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IndexingRepository for PostgresIndexingRepository {
    async fn load(
        &self,
        indexing_id: Uuid,
    ) -> Result<Option<IndexingReadModel>, IndexingRepositoryError> {
        let row: Option<(serde_json::Value,)> =
            sqlx::query_as("SELECT read_model FROM indexings WHERE indexing_id = $1")
                .bind(indexing_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| IndexingRepositoryError::Internal(format!("load: {e}")))?;

        match row {
            None => Ok(None),
            Some((json,)) => serde_json::from_value(json)
                .map(Some)
                .map_err(|e| IndexingRepositoryError::Internal(format!("deserialize: {e}"))),
        }
    }

    async fn save(&self, read_model: IndexingReadModel) -> Result<(), IndexingRepositoryError> {
        let json = serde_json::to_value(&read_model)
            .map_err(|e| IndexingRepositoryError::Internal(format!("serialize: {e}")))?;

        sqlx::query(
            r#"
            INSERT INTO indexings (indexing_id, document_id, read_model, updated_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (indexing_id) DO UPDATE SET
                read_model = $3,
                updated_at = NOW()
            "#,
        )
        .bind(read_model.indexing_id)
        .bind(read_model.document_id)
        .bind(&json)
        .execute(&self.pool)
        .await
        .map_err(|e| IndexingRepositoryError::Internal(format!("save: {e}")))?;

        Ok(())
    }

    async fn list_for_document(
        &self,
        document_id: Uuid,
    ) -> Result<Vec<IndexingReadModel>, IndexingRepositoryError> {
        let rows: Vec<(serde_json::Value,)> = sqlx::query_as(
            "SELECT read_model FROM indexings WHERE document_id = $1 ORDER BY updated_at DESC",
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| IndexingRepositoryError::Internal(format!("list_for_document: {e}")))?;

        rows.into_iter()
            .map(|(json,)| {
                serde_json::from_value(json)
                    .map_err(|e| IndexingRepositoryError::Internal(format!("deserialize: {e}")))
            })
            .collect()
    }
}
