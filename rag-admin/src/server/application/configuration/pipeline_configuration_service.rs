use std::sync::Arc;

use uuid::Uuid;

use crate::server::application::AppError;
use crate::server::domain::configuration::pipeline_configuration::{
    NewPipelineConfiguration, PipelineConfigurationRepository, PipelineConfigurationUpdate,
};
use crate::shared::{
    CreatePipelineConfigurationDto, DeletePipelineConfigurationDto, PipelineConfigurationCommandDto,
    UpdatePipelineConfigurationDto,
};

pub struct PipelineConfigurationService {
    repository: Arc<dyn PipelineConfigurationRepository>,
}

impl PipelineConfigurationService {
    pub fn new(repository: Arc<dyn PipelineConfigurationRepository>) -> Arc<Self> {
        Arc::new(Self { repository })
    }

    pub async fn handle_dto(
        &self,
        command: PipelineConfigurationCommandDto,
    ) -> Result<(), AppError> {
        match command {
            PipelineConfigurationCommandDto::CreatePipelineConfiguration(d) => self.create(d).await,
            PipelineConfigurationCommandDto::UpdatePipelineConfiguration(d) => self.update(d).await,
            PipelineConfigurationCommandDto::DeletePipelineConfiguration(d) => self.delete(d).await,
        }
    }

    async fn create(&self, dto: CreatePipelineConfigurationDto) -> Result<(), AppError> {
        validate_non_empty("pipeline configuration name", &dto.name)?;
        self.repository
            .create(NewPipelineConfiguration {
                id: Uuid::new_v4(),
                name: dto.name,
                embedding_model_id: dto.embedding_model_id,
                generation_model_id: dto.generation_model_id,
                vector_index_id: dto.vector_index_id,
            })
            .await?;
        Ok(())
    }

    async fn update(&self, dto: UpdatePipelineConfigurationDto) -> Result<(), AppError> {
        validate_non_empty("pipeline configuration name", &dto.name)?;
        self.repository
            .update(PipelineConfigurationUpdate {
                id: dto.pipeline_configuration_id,
                name: dto.name,
                embedding_model_id: dto.embedding_model_id,
                generation_model_id: dto.generation_model_id,
                vector_index_id: dto.vector_index_id,
            })
            .await?;
        Ok(())
    }

    async fn delete(&self, dto: DeletePipelineConfigurationDto) -> Result<(), AppError> {
        self.repository
            .delete(dto.pipeline_configuration_id)
            .await?;
        Ok(())
    }
}

fn validate_non_empty(field: &str, value: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!("{field} cannot be empty")));
    }
    Ok(())
}
