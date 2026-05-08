use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::server::domain::{
    ai_provider::entity::AiProvdier,
    embedding_model::entity::EmbeddingModel,
    generation_model::entity::GenerationModel,
    pipeline_configuration::{
        PipelineConfiguration, PipelineConfigurationRepository,
        PipelineConfigurationRepositoryError,
    },
    vector_index::entity::VectorIndex,
    vector_store_provider::entity::VectorStoreProvider,
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
    async fn load(&self) -> Result<PipelineConfiguration, PipelineConfigurationRepositoryError> {
        let row: Option<PipelineConfigurationRow> = sqlx::query_as(
            r#"
            SELECT id,
                   ai_providers,
                   vector_store_providers,
                   embedding_models,
                   generation_models,
                   vector_indexes,
                   current_embedding_model_id,
                   current_generation_model_id,
                   current_vector_index_id
            FROM pipeline_configuration
            WHERE id = $1
            "#,
        )
        .bind(PipelineConfiguration::default().configuration_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PipelineConfigurationRepositoryError::Internal(format!("load: {e}")))?;

        match row {
            None => Ok(PipelineConfiguration::default()),
            Some(row) => row.try_into(),
        }
    }

    async fn save(
        &self,
        config: PipelineConfiguration,
    ) -> Result<(), PipelineConfigurationRepositoryError> {
        let ai_providers = serde_json::to_value(&config.ai_providers).map_err(|e| {
            PipelineConfigurationRepositoryError::Internal(format!("serialize ai_providers: {e}"))
        })?;
        let vector_store_providers =
            serde_json::to_value(&config.vector_store_providers).map_err(|e| {
                PipelineConfigurationRepositoryError::Internal(format!(
                    "serialize vector_store_providers: {e}"
                ))
            })?;
        let embedding_models = serde_json::to_value(&config.embedding_models).map_err(|e| {
            PipelineConfigurationRepositoryError::Internal(format!(
                "serialize embedding_models: {e}"
            ))
        })?;
        let generation_models = serde_json::to_value(&config.generation_models).map_err(|e| {
            PipelineConfigurationRepositoryError::Internal(format!(
                "serialize generation_models: {e}"
            ))
        })?;
        let vector_indexes = serde_json::to_value(&config.vector_indexes).map_err(|e| {
            PipelineConfigurationRepositoryError::Internal(format!("serialize vector_indexes: {e}"))
        })?;

        sqlx::query(
            r#"
            INSERT INTO pipeline_configuration (
                id,
                ai_providers,
                vector_store_providers,
                embedding_models,
                generation_models,
                vector_indexes,
                current_embedding_model_id,
                current_generation_model_id,
                current_vector_index_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO UPDATE SET
                ai_providers                = $2,
                vector_store_providers      = $3,
                embedding_models            = $4,
                generation_models           = $5,
                vector_indexes              = $6,
                current_embedding_model_id  = $7,
                current_generation_model_id = $8,
                current_vector_index_id     = $9,
                updated_at                  = NOW()
            "#,
        )
        .bind(config.configuration_id)
        .bind(&ai_providers)
        .bind(&vector_store_providers)
        .bind(&embedding_models)
        .bind(&generation_models)
        .bind(&vector_indexes)
        .bind(config.current_embedding_model_id)
        .bind(config.current_generation_model_id)
        .bind(config.current_vector_index_id)
        .execute(&self.pool)
        .await
        .map_err(|e| PipelineConfigurationRepositoryError::Internal(format!("save: {e}")))?;

        Ok(())
    }
}

struct PipelineConfigurationRow {
    id: Uuid,
    ai_providers: serde_json::Value,
    vector_store_providers: serde_json::Value,
    embedding_models: serde_json::Value,
    generation_models: serde_json::Value,
    vector_indexes: serde_json::Value,
    current_embedding_model_id: Option<Uuid>,
    current_generation_model_id: Option<Uuid>,
    current_vector_index_id: Option<Uuid>,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for PipelineConfigurationRow {
    fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            ai_providers: row.try_get("ai_providers")?,
            vector_store_providers: row.try_get("vector_store_providers")?,
            embedding_models: row.try_get("embedding_models")?,
            generation_models: row.try_get("generation_models")?,
            vector_indexes: row.try_get("vector_indexes")?,
            current_embedding_model_id: row.try_get("current_embedding_model_id")?,
            current_generation_model_id: row.try_get("current_generation_model_id")?,
            current_vector_index_id: row.try_get("current_vector_index_id")?,
        })
    }
}

impl TryFrom<PipelineConfigurationRow> for PipelineConfiguration {
    type Error = PipelineConfigurationRepositoryError;

    fn try_from(row: PipelineConfigurationRow) -> Result<Self, Self::Error> {
        let ai_providers: Vec<AiProvdier> =
            serde_json::from_value(row.ai_providers).map_err(|e| {
                PipelineConfigurationRepositoryError::Internal(format!(
                    "deserialize ai_providers: {e}"
                ))
            })?;
        let vector_store_providers: Vec<VectorStoreProvider> =
            serde_json::from_value(row.vector_store_providers).map_err(|e| {
                PipelineConfigurationRepositoryError::Internal(format!(
                    "deserialize vector_store_providers: {e}"
                ))
            })?;
        let embedding_models: Vec<EmbeddingModel> = serde_json::from_value(row.embedding_models)
            .map_err(|e| {
                PipelineConfigurationRepositoryError::Internal(format!(
                    "deserialize embedding_models: {e}"
                ))
            })?;
        let generation_models: Vec<GenerationModel> = serde_json::from_value(row.generation_models)
            .map_err(|e| {
                PipelineConfigurationRepositoryError::Internal(format!(
                    "deserialize generation_models: {e}"
                ))
            })?;
        let vector_indexes: Vec<VectorIndex> =
            serde_json::from_value(row.vector_indexes).map_err(|e| {
                PipelineConfigurationRepositoryError::Internal(format!(
                    "deserialize vector_indexes: {e}"
                ))
            })?;

        Ok(PipelineConfiguration {
            configuration_id: row.id,
            ai_providers,
            vector_store_providers,
            embedding_models,
            generation_models,
            vector_indexes,
            current_embedding_model_id: row.current_embedding_model_id,
            current_generation_model_id: row.current_generation_model_id,
            current_vector_index_id: row.current_vector_index_id,
        })
    }
}
