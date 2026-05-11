use std::collections::HashMap;
use std::sync::Arc;

use uuid::Uuid;

use crate::server::application::ports::{
    ChatClient, ChatRequest, ChatResponse, ChatResponseFormat,
};
use crate::server::application::AppError;
use crate::server::domain::configuration::aggregate::Configuration;
use crate::server::domain::configuration::kinds::AiProviderKind;
use crate::server::event_sourcing::{Aggregate, AggregateRepository};

#[derive(Debug, Clone)]
pub struct ResolvedGenerationModel {
    pub generation_model_id: Uuid,
    pub kind: AiProviderKind,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct ChatPrompt {
    pub system: String,
    pub user: String,
    pub temperature: f32,
    pub response_format: ChatResponseFormat,
}

pub struct ChatService {
    clients: HashMap<AiProviderKind, Arc<dyn ChatClient>>,
    configuration_repository: Arc<AggregateRepository<Configuration>>,
}

impl ChatService {
    pub fn new(
        clients: HashMap<AiProviderKind, Arc<dyn ChatClient>>,
        configuration_repository: Arc<AggregateRepository<Configuration>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            clients,
            configuration_repository,
        })
    }

    pub async fn chat(
        &self,
        generation_model_id: Uuid,
        prompt: ChatPrompt,
    ) -> Result<ChatResponse, AppError> {
        let resolved = self.resolve(generation_model_id).await?;
        let client = self.clients.get(&resolved.kind).ok_or_else(|| {
            AppError::Internal(format!(
                "no chat client registered for provider kind {}",
                resolved.kind.as_str()
            ))
        })?;
        client
            .chat(ChatRequest {
                model: resolved.model,
                system: prompt.system,
                user: prompt.user,
                temperature: prompt.temperature,
                response_format: prompt.response_format,
            })
            .await
    }

    pub async fn resolve(
        &self,
        generation_model_id: Uuid,
    ) -> Result<ResolvedGenerationModel, AppError> {
        let Some(loaded_aggregate) = self
            .configuration_repository
            .load(Configuration::singleton_id())
            .await?
        else {
            return Err(AppError::NotFound(
                Configuration::aggregate_type().to_string(),
            ));
        };
        let model = loaded_aggregate
            .aggregate
            .generation_models
            .iter()
            .find(|m| m.generation_model_id == generation_model_id)
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "generation model {generation_model_id} not registered"
                ))
            })?;
        Ok(ResolvedGenerationModel {
            generation_model_id: model.generation_model_id,
            kind: model.kind,
            model: model.model.clone(),
        })
    }
}
