use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::server::domain::configuration::chunking_configuration::{
    ChunkingConfigurationReadModel, ChunkingConfigurationRepository,
    ChunkingConfigurationRepositoryError,
};
use crate::shared::ChunkingConfig;

pub struct PostgresChunkingConfigurationRepository {
    pool: PgPool,
}

impl PostgresChunkingConfigurationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ChunkingConfigurationRepository for PostgresChunkingConfigurationRepository {
    async fn load_all(
        &self,
    ) -> Result<Vec<ChunkingConfigurationReadModel>, ChunkingConfigurationRepositoryError> {
        let rows: Vec<ChunkingConfigurationRow> = sqlx::query_as(
            r#"
            SELECT id, name, config
            FROM chunking_configurations
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ChunkingConfigurationRepositoryError::Internal(format!("load_all: {e}")))?;

        rows.into_iter()
            .map(ChunkingConfigurationRow::try_into)
            .collect()
    }

    async fn save(
        &self,
        read_model: ChunkingConfigurationReadModel,
    ) -> Result<(), ChunkingConfigurationRepositoryError> {
        let config_json = serde_json::to_value(&read_model.config).map_err(|e| {
            ChunkingConfigurationRepositoryError::Internal(format!("serialize config: {e}"))
        })?;

        sqlx::query(
            r#"
            INSERT INTO chunking_configurations (id, name, config)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE SET
                name       = $2,
                config     = $3,
                updated_at = NOW()
            "#,
        )
        .bind(read_model.chunking_configuration_id)
        .bind(&read_model.name)
        .bind(&config_json)
        .execute(&self.pool)
        .await
        .map_err(|e| ChunkingConfigurationRepositoryError::Internal(format!("save: {e}")))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), ChunkingConfigurationRepositoryError> {
        sqlx::query("DELETE FROM chunking_configurations WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| ChunkingConfigurationRepositoryError::Internal(format!("delete: {e}")))?;

        Ok(())
    }
}

struct ChunkingConfigurationRow {
    id: Uuid,
    name: String,
    config: serde_json::Value,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for ChunkingConfigurationRow {
    fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            config: row.try_get("config")?,
        })
    }
}

impl TryFrom<ChunkingConfigurationRow> for ChunkingConfigurationReadModel {
    type Error = ChunkingConfigurationRepositoryError;

    fn try_from(row: ChunkingConfigurationRow) -> Result<Self, Self::Error> {
        let config: ChunkingConfig = serde_json::from_value(row.config).map_err(|e| {
            ChunkingConfigurationRepositoryError::Internal(format!("deserialize config: {e}"))
        })?;
        Ok(Self {
            chunking_configuration_id: row.id,
            name: row.name,
            config,
        })
    }
}
