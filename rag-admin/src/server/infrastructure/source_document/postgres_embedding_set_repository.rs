use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::server::application::{source_document::ports::EmbeddingSetRepository, AppError};
use crate::server::domain::embedding_set::entity::{ChunkEmbedding, EmbeddingSet};

pub struct PostgresEmbeddingSetRepository {
    pool: PgPool,
}

impl PostgresEmbeddingSetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EmbeddingSetRepository for PostgresEmbeddingSetRepository {
    async fn save(
        &self,
        embedding_set: EmbeddingSet,
        embeddings: Vec<ChunkEmbedding>,
    ) -> Result<(), AppError> {
        let model_snapshot = serde_json::to_value(&embedding_set.embedding_model_snapshot)
            .map_err(|e| AppError::Internal(format!("serialize model_snapshot: {e}")))?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(format!("begin transaction: {e}")))?;

        sqlx::query(
            r#"
            INSERT INTO embedding_sets (
                embedding_set_id, chunk_set_id, embedding_model_id,
                embedding_model_snapshot, dimensions, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (chunk_set_id, embedding_model_id) DO NOTHING
            "#,
        )
        .bind(embedding_set.embedding_set_id)
        .bind(embedding_set.chunk_set_id)
        .bind(embedding_set.embedding_model_id)
        .bind(&model_snapshot)
        .bind(embedding_set.dimensions as i32)
        .bind(&embedding_set.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(format!("save embedding_set: {e}")))?;

        for embedding in &embeddings {
            // REAL[] is stored as a JSON array for portability; migrate to pgvector later.
            let vector_json = serde_json::to_value(&embedding.vector)
                .map_err(|e| AppError::Internal(format!("serialize vector: {e}")))?;

            sqlx::query(
                r#"
                INSERT INTO chunk_embeddings (chunk_id, embedding_set_id, vector)
                VALUES ($1, $2, $3)
                ON CONFLICT (chunk_id, embedding_set_id) DO NOTHING
                "#,
            )
            .bind(embedding.chunk_id)
            .bind(embedding.embedding_set_id)
            .bind(&vector_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(format!("save chunk_embedding: {e}")))?;
        }

        tx.commit()
            .await
            .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

        Ok(())
    }

    async fn load(&self, embedding_set_id: Uuid) -> Result<Option<EmbeddingSet>, AppError> {
        let row: Option<EmbeddingSetRow> = sqlx::query_as(
            r#"SELECT embedding_set_id, chunk_set_id, embedding_model_id,
                      embedding_model_snapshot, dimensions, created_at
               FROM embedding_sets WHERE embedding_set_id = $1"#,
        )
        .bind(embedding_set_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("load embedding_set: {e}")))?;

        row.map(EmbeddingSet::try_from).transpose()
    }

    async fn find_by(
        &self,
        chunk_set_id: Uuid,
        embedding_model_id: Uuid,
    ) -> Result<Option<EmbeddingSet>, AppError> {
        let row: Option<EmbeddingSetRow> = sqlx::query_as(
            r#"SELECT embedding_set_id, chunk_set_id, embedding_model_id,
                      embedding_model_snapshot, dimensions, created_at
               FROM embedding_sets
               WHERE chunk_set_id = $1 AND embedding_model_id = $2"#,
        )
        .bind(chunk_set_id)
        .bind(embedding_model_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("find_by: {e}")))?;

        row.map(EmbeddingSet::try_from).transpose()
    }

    async fn load_embeddings(
        &self,
        embedding_set_id: Uuid,
    ) -> Result<Vec<ChunkEmbedding>, AppError> {
        let rows: Vec<EmbeddingRow> = sqlx::query_as(
            "SELECT chunk_id, embedding_set_id, vector FROM chunk_embeddings WHERE embedding_set_id = $1",
        )
        .bind(embedding_set_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("load_embeddings: {e}")))?;

        rows.into_iter()
            .map(ChunkEmbedding::try_from)
            .collect()
    }
}

struct EmbeddingSetRow {
    embedding_set_id: Uuid,
    chunk_set_id: Uuid,
    embedding_model_id: Uuid,
    embedding_model_snapshot: serde_json::Value,
    dimensions: i32,
    created_at: String,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for EmbeddingSetRow {
    fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            embedding_set_id: row.try_get("embedding_set_id")?,
            chunk_set_id: row.try_get("chunk_set_id")?,
            embedding_model_id: row.try_get("embedding_model_id")?,
            embedding_model_snapshot: row.try_get("embedding_model_snapshot")?,
            dimensions: row.try_get("dimensions")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

impl TryFrom<EmbeddingSetRow> for EmbeddingSet {
    type Error = AppError;

    fn try_from(row: EmbeddingSetRow) -> Result<Self, Self::Error> {
        Ok(EmbeddingSet {
            embedding_set_id: row.embedding_set_id,
            chunk_set_id: row.chunk_set_id,
            embedding_model_id: row.embedding_model_id,
            embedding_model_snapshot: serde_json::from_value(row.embedding_model_snapshot)
                .map_err(|e| {
                    AppError::Internal(format!("deserialize model_snapshot: {e}"))
                })?,
            dimensions: row.dimensions as u32,
            created_at: row.created_at,
        })
    }
}

struct EmbeddingRow {
    chunk_id: Uuid,
    embedding_set_id: Uuid,
    vector: serde_json::Value,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for EmbeddingRow {
    fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            chunk_id: row.try_get("chunk_id")?,
            embedding_set_id: row.try_get("embedding_set_id")?,
            vector: row.try_get("vector")?,
        })
    }
}

impl TryFrom<EmbeddingRow> for ChunkEmbedding {
    type Error = AppError;

    fn try_from(row: EmbeddingRow) -> Result<Self, Self::Error> {
        Ok(ChunkEmbedding {
            chunk_id: row.chunk_id,
            embedding_set_id: row.embedding_set_id,
            vector: serde_json::from_value(row.vector)
                .map_err(|e| AppError::Internal(format!("deserialize vector: {e}")))?,
        })
    }
}
