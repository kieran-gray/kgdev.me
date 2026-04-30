use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::error;
use worker::Ai;

use crate::api_worker::application::{AiInferenceServiceTrait, AppError};

pub struct WorkersAiService {
    ai: Ai,
    embedding_model: String,
    generation_model: String,
}

impl WorkersAiService {
    pub fn create(ai: Ai, embedding_model: String, generation_model: String) -> Arc<Self> {
        Arc::new(Self {
            ai,
            embedding_model,
            generation_model,
        })
    }
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    text: Vec<&'a str>,
}

#[derive(Deserialize)]
struct EmbedResponse {
    data: Vec<Vec<f32>>,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    messages: Vec<ChatMessage<'a>>,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    response: String,
}

#[async_trait(?Send)]
impl AiInferenceServiceTrait for WorkersAiService {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AppError> {
        let request = EmbedRequest { text: vec![text] };
        let response: EmbedResponse =
            self.ai
                .run(&self.embedding_model, request)
                .await
                .map_err(|e| {
                    error!(error = %e, "Workers AI embed failed");
                    AppError::InternalError(format!("Embedding failed: {e}"))
                })?;

        response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| AppError::InternalError("Embedding response had no vectors".to_string()))
    }

    async fn generate(&self, system: &str, user: &str) -> Result<String, AppError> {
        let request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: system,
                },
                ChatMessage {
                    role: "user",
                    content: user,
                },
            ],
        };

        let response: ChatResponse =
            self.ai
                .run(&self.generation_model, request)
                .await
                .map_err(|e| {
                    error!(error = %e, "Workers AI generation failed");
                    AppError::InternalError(format!("Generation failed: {e}"))
                })?;

        let trimmed = response.response.trim();
        if trimmed.is_empty() {
            return Err(AppError::InternalError(
                "Generation returned empty response".to_string(),
            ));
        }
        Ok(trimmed.to_string())
    }
}
