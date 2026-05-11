use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::server::domain::configuration::pipeline_configuration::{
    PipelineConfigurationReadModel, PipelineConfigurationRepository,
    PipelineConfigurationRepositoryError,
};

pub struct PostgresPipelineConfigurationRepository {
    pool: PgPool,
}

impl PostgresPipelineConfigurationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PipelineConfigurationRepository for PostgresPipelineConfigurationRepository {
    async fn load_all(
        &self,
    ) -> Result<Vec<PipelineConfigurationReadModel>, PipelineConfigurationRepositoryError> {
        let rows: Vec<PipelineConfigurationRow> = sqlx::query_as(
            r#"
            SELECT id, name, embedding_model_id, generation_model_id, vector_index_id
            FROM pipeline_configurations
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PipelineConfigurationRepositoryError::Internal(format!("load_all: {e}")))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn save(
        &self,
        read_model: PipelineConfigurationReadModel,
    ) -> Result<(), PipelineConfigurationRepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO pipeline_configurations (
                id, name, embedding_model_id, generation_model_id, vector_index_id
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE SET
                name                = $2,
                embedding_model_id  = $3,
                generation_model_id = $4,
                vector_index_id     = $5,
                updated_at          = NOW()
            "#,
        )
        .bind(read_model.pipeline_configuration_id)
        .bind(&read_model.name)
        .bind(read_model.embedding_model_id)
        .bind(read_model.generation_model_id)
        .bind(read_model.vector_index_id)
        .execute(&self.pool)
        .await
        .map_err(|e| PipelineConfigurationRepositoryError::Internal(format!("save: {e}")))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), PipelineConfigurationRepositoryError> {
        sqlx::query("DELETE FROM pipeline_configurations WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| PipelineConfigurationRepositoryError::Internal(format!("delete: {e}")))?;

        Ok(())
    }
}

struct PipelineConfigurationRow {
    id: Uuid,
    name: String,
    embedding_model_id: Uuid,
    generation_model_id: Uuid,
    vector_index_id: Uuid,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for PipelineConfigurationRow {
    fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            embedding_model_id: row.try_get("embedding_model_id")?,
            generation_model_id: row.try_get("generation_model_id")?,
            vector_index_id: row.try_get("vector_index_id")?,
        })
    }
}

impl From<PipelineConfigurationRow> for PipelineConfigurationReadModel {
    fn from(row: PipelineConfigurationRow) -> Self {
        Self {
            pipeline_configuration_id: row.id,
            name: row.name,
            embedding_model_id: row.embedding_model_id,
            generation_model_id: row.generation_model_id,
            vector_index_id: row.vector_index_id,
        }
    }
}
