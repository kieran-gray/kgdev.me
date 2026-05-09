use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::server::domain::{
    configuration::{
        read_model::ConfigurationReadModel,
        repository::{ConfigurationRepository, ConfigurationRepositoryError},
    },
    embedding_model::entity::EmbeddingModel,
    generation_model::entity::GenerationModel,
    vector_index::entity::VectorIndex,
};

pub struct PostgresConfigurationRepository {
    pool: PgPool,
}

impl PostgresConfigurationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ConfigurationRepository for PostgresConfigurationRepository {
    async fn load(&self) -> Result<ConfigurationReadModel, ConfigurationRepositoryError> {
        let row: Option<ConfigurationRow> = sqlx::query_as(
            r#"
            SELECT id,
                   ai_providers,
                   vector_store_providers,
                   embedding_models,
                   generation_models,
                   vector_indexes
            FROM configuration
            WHERE id = $1
            "#,
        )
        .bind(Uuid::nil())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ConfigurationRepositoryError::Internal(format!("load: {e}")))?;

        match row {
            None => Ok(ConfigurationReadModel::default()),
            Some(row) => row.try_into(),
        }
    }

    async fn save(
        &self,
        read_model: ConfigurationReadModel,
    ) -> Result<(), ConfigurationRepositoryError> {
        let ai_providers = serde_json::to_value(&read_model.ai_providers).map_err(|e| {
            ConfigurationRepositoryError::Internal(format!("serialize ai_providers: {e}"))
        })?;
        let vector_store_providers = serde_json::to_value(&read_model.vector_store_providers)
            .map_err(|e| {
                ConfigurationRepositoryError::Internal(format!(
                    "serialize vector_store_providers: {e}"
                ))
            })?;
        let embedding_models = serde_json::to_value(&read_model.embedding_models).map_err(|e| {
            ConfigurationRepositoryError::Internal(format!("serialize embedding_models: {e}"))
        })?;
        let generation_models =
            serde_json::to_value(&read_model.generation_models).map_err(|e| {
                ConfigurationRepositoryError::Internal(format!("serialize generation_models: {e}"))
            })?;
        let vector_indexes = serde_json::to_value(&read_model.vector_indexes).map_err(|e| {
            ConfigurationRepositoryError::Internal(format!("serialize vector_indexes: {e}"))
        })?;

        sqlx::query(
            r#"
            INSERT INTO configuration (
                id,
                ai_providers,
                vector_store_providers,
                embedding_models,
                generation_models,
                vector_indexes
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE SET
                ai_providers           = $2,
                vector_store_providers = $3,
                embedding_models       = $4,
                generation_models      = $5,
                vector_indexes         = $6,
                updated_at             = NOW()
            "#,
        )
        .bind(read_model.configuration_id)
        .bind(&ai_providers)
        .bind(&vector_store_providers)
        .bind(&embedding_models)
        .bind(&generation_models)
        .bind(&vector_indexes)
        .execute(&self.pool)
        .await
        .map_err(|e| ConfigurationRepositoryError::Internal(format!("save: {e}")))?;

        Ok(())
    }
}

struct ConfigurationRow {
    id: Uuid,
    ai_providers: serde_json::Value,
    vector_store_providers: serde_json::Value,
    embedding_models: serde_json::Value,
    generation_models: serde_json::Value,
    vector_indexes: serde_json::Value,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for ConfigurationRow {
    fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            ai_providers: row.try_get("ai_providers")?,
            vector_store_providers: row.try_get("vector_store_providers")?,
            embedding_models: row.try_get("embedding_models")?,
            generation_models: row.try_get("generation_models")?,
            vector_indexes: row.try_get("vector_indexes")?,
        })
    }
}

impl TryFrom<ConfigurationRow> for ConfigurationReadModel {
    type Error = ConfigurationRepositoryError;

    fn try_from(row: ConfigurationRow) -> Result<Self, Self::Error> {
        Ok(ConfigurationReadModel {
            configuration_id: row.id,
            ai_providers: serde_json::from_value(row.ai_providers).map_err(|e| {
                ConfigurationRepositoryError::Internal(format!("deserialize ai_providers: {e}"))
            })?,
            vector_store_providers: serde_json::from_value(row.vector_store_providers).map_err(
                |e| {
                    ConfigurationRepositoryError::Internal(format!(
                        "deserialize vector_store_providers: {e}"
                    ))
                },
            )?,
            embedding_models: serde_json::from_value::<Vec<EmbeddingModel>>(row.embedding_models)
                .map_err(|e| {
                ConfigurationRepositoryError::Internal(format!("deserialize embedding_models: {e}"))
            })?,
            generation_models: serde_json::from_value::<Vec<GenerationModel>>(
                row.generation_models,
            )
            .map_err(|e| {
                ConfigurationRepositoryError::Internal(format!(
                    "deserialize generation_models: {e}"
                ))
            })?,
            vector_indexes: serde_json::from_value::<Vec<VectorIndex>>(row.vector_indexes)
                .map_err(|e| {
                    ConfigurationRepositoryError::Internal(format!(
                        "deserialize vector_indexes: {e}"
                    ))
                })?,
        })
    }
}
