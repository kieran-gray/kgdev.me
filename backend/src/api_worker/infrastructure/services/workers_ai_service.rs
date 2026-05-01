use std::sync::Arc;

use async_stream::stream;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::error;
use worker::Ai;

use crate::api_worker::application::{AiInferenceServiceTrait, AppError, TokenStream};

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
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatStreamChunk {
    response: Option<String>,
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

    async fn generate_stream(&self, system: &str, user: &str) -> Result<TokenStream, AppError> {
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
            stream: true,
        };

        let bytes = self
            .ai
            .run_bytes(&self.generation_model, request)
            .await
            .map_err(|e| {
                error!(error = %e, "Workers AI streaming failed");
                AppError::InternalError(format!("Generation failed: {e}"))
            })?;

        let token_stream = stream! {
            let mut bytes = bytes;
            let mut buffer = String::new();
            let mut emitted_any = false;
            while let Some(chunk) = bytes.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        error!(error = %e, "AI byte stream error");
                        yield Err(AppError::InternalError("Generation stream error".to_string()));
                        return;
                    }
                };
                let text = match std::str::from_utf8(&chunk) {
                    Ok(s) => s,
                    Err(e) => {
                        error!(error = %e, "AI stream non-utf8 chunk");
                        yield Err(AppError::InternalError("Generation stream error".to_string()));
                        return;
                    }
                };
                buffer.push_str(text);
                while let Some(idx) = buffer.find("\n\n") {
                    let event = buffer[..idx].to_string();
                    buffer.drain(..idx + 2);
                    for line in event.lines() {
                        let Some(data) = line.strip_prefix("data:") else { continue };
                        let data = data.trim_start();
                        if data.is_empty() || data == "[DONE]" {
                            continue;
                        }
                        match serde_json::from_str::<ChatStreamChunk>(data) {
                            Ok(parsed) => {
                                if let Some(token) = parsed.response && !token.is_empty() {
                                        emitted_any = true;
                                        yield Ok(token);
                                    }
                                }
                            Err(e) => {
                                error!(error = %e, raw = %data, "AI stream parse failed");
                            }
                        }
                    }
                }
            }
            if !emitted_any {
                yield Err(AppError::InternalError(
                    "Generation produced no tokens".to_string(),
                ));
            }
        };

        Ok(Box::pin(token_stream))
    }
}
