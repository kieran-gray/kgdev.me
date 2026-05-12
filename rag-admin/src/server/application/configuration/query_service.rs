use std::sync::Arc;

use crate::server::application::AppError;
use crate::server::domain::configuration::chunking_configuration::{
    ChunkingConfigurationRepository, ChunkingConfigurationRepositoryError,
};
use crate::server::domain::configuration::pipeline_configuration::{
    PipelineConfigurationRepository, PipelineConfigurationRepositoryError,
};
use crate::server::domain::configuration::sweep_template::{
    SweepTemplateRepository, SweepTemplateRepositoryError,
};
use crate::server::domain::configuration::{ConfigurationRepository, ConfigurationRepositoryError};
use crate::shared::{
    ChunkingConfigurationDto, ConfigurationDto, PipelineConfigurationDto, SweepTemplateDto,
};

pub struct ConfigurationQueryService {
    repository: Arc<dyn ConfigurationRepository>,
}

impl ConfigurationQueryService {
    pub fn new(repository: Arc<dyn ConfigurationRepository>) -> Arc<Self> {
        Arc::new(Self { repository })
    }

    pub async fn get(&self) -> Result<ConfigurationDto, AppError> {
        self.repository
            .load()
            .await
            .map(|ref c| c.into())
            .map_err(|e| match e {
                ConfigurationRepositoryError::Internal(m) => AppError::Internal(m),
            })
    }
}

pub struct PipelineConfigurationQueryService {
    pipeline_repository: Arc<dyn PipelineConfigurationRepository>,
    configuration_repository: Arc<dyn ConfigurationRepository>,
}

impl PipelineConfigurationQueryService {
    pub fn new(
        pipeline_repository: Arc<dyn PipelineConfigurationRepository>,
        configuration_repository: Arc<dyn ConfigurationRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            pipeline_repository,
            configuration_repository,
        })
    }

    pub async fn list(&self) -> Result<Vec<PipelineConfigurationDto>, AppError> {
        let pipeline_configs = self
            .pipeline_repository
            .load_all()
            .await
            .map_err(|e| match e {
                PipelineConfigurationRepositoryError::Internal(m) => AppError::Internal(m),
            })?;

        let catalog = self
            .configuration_repository
            .load()
            .await
            .map_err(|e| match e {
                ConfigurationRepositoryError::Internal(m) => AppError::Internal(m),
            })?;

        Ok(pipeline_configs
            .iter()
            .map(|pc| (pc, &catalog).into())
            .collect())
    }
}

pub struct ChunkingConfigurationQueryService {
    repository: Arc<dyn ChunkingConfigurationRepository>,
}

impl ChunkingConfigurationQueryService {
    pub fn new(repository: Arc<dyn ChunkingConfigurationRepository>) -> Arc<Self> {
        Arc::new(Self { repository })
    }

    pub async fn list(&self) -> Result<Vec<ChunkingConfigurationDto>, AppError> {
        let read_models = self.repository.load_all().await.map_err(|e| match e {
            ChunkingConfigurationRepositoryError::Internal(m) => AppError::Internal(m),
        })?;

        Ok(read_models
            .into_iter()
            .map(|cc| ChunkingConfigurationDto {
                chunking_configuration_id: cc.chunking_configuration_id,
                name: cc.name,
                config: cc.config,
            })
            .collect())
    }
}

pub struct SweepTemplateQueryService {
    repository: Arc<dyn SweepTemplateRepository>,
}

impl SweepTemplateQueryService {
    pub fn new(repository: Arc<dyn SweepTemplateRepository>) -> Arc<Self> {
        Arc::new(Self { repository })
    }

    pub async fn list(&self) -> Result<Vec<SweepTemplateDto>, AppError> {
        let read_models = self.repository.load_all().await.map_err(|e| match e {
            SweepTemplateRepositoryError::Internal(m) => AppError::Internal(m),
        })?;

        Ok(read_models
            .into_iter()
            .map(|st| SweepTemplateDto {
                sweep_template_id: st.sweep_template_id,
                name: st.name,
                members: st.members,
                is_default: st.is_default,
            })
            .collect())
    }
}
