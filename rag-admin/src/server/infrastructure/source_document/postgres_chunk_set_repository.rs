use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::server::application::{source_document::ports::ChunkSetRepository, AppError};
use crate::server::domain::chunk_set::entity::{Chunk, ChunkSet};

pub struct PostgresChunkSetRepository {
    pool: PgPool,
}

impl PostgresChunkSetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ChunkSetRepository for PostgresChunkSetRepository {
    async fn save(&self, chunk_set: ChunkSet, chunks: Vec<Chunk>) -> Result<(), AppError> {
        let chunking_config = serde_json::to_value(&chunk_set.chunking_config)
            .map_err(|e| AppError::Internal(format!("serialize chunking_config: {e}")))?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(format!("begin transaction: {e}")))?;

        sqlx::query(
            r#"
            INSERT INTO chunk_sets (chunk_set_id, document_id, document_version, chunking_config, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (chunk_set_id) DO NOTHING
            "#,
        )
        .bind(chunk_set.chunk_set_id)
        .bind(chunk_set.document_id)
        .bind(chunk_set.document_version as i32)
        .bind(&chunking_config)
        .bind(&chunk_set.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(format!("save chunk_set: {e}")))?;

        for chunk in &chunks {
            sqlx::query(
                r#"
                INSERT INTO chunks (chunk_id, chunk_set_id, sequence, heading, text, char_start, char_end)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (chunk_id) DO NOTHING
                "#,
            )
            .bind(chunk.chunk_id)
            .bind(chunk.chunk_set_id)
            .bind(chunk.sequence as i32)
            .bind(&chunk.heading)
            .bind(&chunk.text)
            .bind(chunk.char_start as i32)
            .bind(chunk.char_end as i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(format!("save chunk: {e}")))?;
        }

        tx.commit()
            .await
            .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

        Ok(())
    }

    async fn load(&self, chunk_set_id: Uuid) -> Result<Option<ChunkSet>, AppError> {
        let row: Option<ChunkSetRow> = sqlx::query_as(
            "SELECT chunk_set_id, document_id, document_version, chunking_config, created_at FROM chunk_sets WHERE chunk_set_id = $1",
        )
        .bind(chunk_set_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("load chunk_set: {e}")))?;

        row.map(ChunkSet::try_from).transpose()
    }

    async fn load_chunks(&self, chunk_set_id: Uuid) -> Result<Vec<Chunk>, AppError> {
        let rows: Vec<ChunkRow> = sqlx::query_as(
            "SELECT chunk_id, chunk_set_id, sequence, heading, text, char_start, char_end FROM chunks WHERE chunk_set_id = $1 ORDER BY sequence ASC",
        )
        .bind(chunk_set_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("load chunks: {e}")))?;

        Ok(rows.into_iter().map(Chunk::from).collect())
    }

    async fn list_for_document(&self, document_id: Uuid) -> Result<Vec<ChunkSet>, AppError> {
        let rows: Vec<ChunkSetRow> = sqlx::query_as(
            "SELECT chunk_set_id, document_id, document_version, chunking_config, created_at FROM chunk_sets WHERE document_id = $1 ORDER BY created_at DESC",
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("list chunk_sets: {e}")))?;

        rows.into_iter()
            .map(ChunkSet::try_from)
            .collect()
    }
}

struct ChunkSetRow {
    chunk_set_id: Uuid,
    document_id: Uuid,
    document_version: i32,
    chunking_config: serde_json::Value,
    created_at: String,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for ChunkSetRow {
    fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            chunk_set_id: row.try_get("chunk_set_id")?,
            document_id: row.try_get("document_id")?,
            document_version: row.try_get("document_version")?,
            chunking_config: row.try_get("chunking_config")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

impl TryFrom<ChunkSetRow> for ChunkSet {
    type Error = AppError;

    fn try_from(row: ChunkSetRow) -> Result<Self, Self::Error> {
        Ok(ChunkSet {
            chunk_set_id: row.chunk_set_id,
            document_id: row.document_id,
            document_version: row.document_version as u32,
            chunking_config: serde_json::from_value(row.chunking_config)
                .map_err(|e| AppError::Internal(format!("deserialize chunking_config: {e}")))?,
            created_at: row.created_at,
        })
    }
}

struct ChunkRow {
    chunk_id: Uuid,
    chunk_set_id: Uuid,
    sequence: i32,
    heading: String,
    text: String,
    char_start: i32,
    char_end: i32,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for ChunkRow {
    fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            chunk_id: row.try_get("chunk_id")?,
            chunk_set_id: row.try_get("chunk_set_id")?,
            sequence: row.try_get("sequence")?,
            heading: row.try_get("heading")?,
            text: row.try_get("text")?,
            char_start: row.try_get("char_start")?,
            char_end: row.try_get("char_end")?,
        })
    }
}

impl From<ChunkRow> for Chunk {
    fn from(row: ChunkRow) -> Self {
        Chunk {
            chunk_id: row.chunk_id,
            chunk_set_id: row.chunk_set_id,
            sequence: row.sequence as u32,
            heading: row.heading,
            text: row.text,
            char_start: row.char_start as u32,
            char_end: row.char_end as u32,
        }
    }
}
