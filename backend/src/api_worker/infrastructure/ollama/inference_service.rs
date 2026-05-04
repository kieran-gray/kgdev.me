use std::sync::Arc;

use async_stream::stream;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::api_worker::{
    application::{AiInferenceServiceTrait, AppError, TokenStream},
    infrastructure::HttpClientTrait,
};

const DEFAULT_EMBEDDING_DIMENSIONS: u32 = 1024;

pub struct OllamaInferenceService {
    client: Arc<dyn HttpClientTrait>,
    embedding_model: String,
    generation_model: String,
    ollama_url: String,
    embedding_dimensions: u32,
}

impl OllamaInferenceService {
    pub fn create(
        client: Arc<dyn HttpClientTrait>,
        embedding_model: String,
        generation_model: String,
        ollama_url: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            client,
            embedding_model,
            generation_model,
            ollama_url: ollama_url.trim_end_matches('/').to_string(),
            embedding_dimensions: DEFAULT_EMBEDDING_DIMENSIONS,
        })
    }
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    input: Vec<&'a str>,
    dimensions: u32,
}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    system: &'a str,
    stream: bool,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: Option<String>,
    done: bool,
}

fn parse_generate_line(line: &[u8]) -> Result<GenerateResponse, AppError> {
    let line = std::str::from_utf8(line).map_err(|e| {
        error!(error = %e, "Ollama stream line was not UTF-8");
        AppError::InternalError("Generation stream error".to_string())
    })?;

    serde_json::from_str::<GenerateResponse>(line).map_err(|e| {
        error!(error = %e, raw = %line, "Ollama stream parse failed");
        AppError::InternalError("Generation stream error".to_string())
    })
}

#[async_trait(?Send)]
impl AiInferenceServiceTrait for OllamaInferenceService {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AppError> {
        let url = format!("{}/api/embed", self.ollama_url);
        let request = EmbedRequest {
            model: &self.embedding_model,
            input: vec![text],
            dimensions: self.embedding_dimensions,
        };
        let body = serde_json::to_value(request).map_err(|e| {
            AppError::InternalError(format!("Failed to convert ollama request to Value: {e}"))
        })?;

        let headers = vec![("Content-Type", "application/json")];

        let response = self
            .client
            .post(&url, body, headers)
            .await
            .map_err(|e| AppError::InternalError(format!("Ollama client failure: {e}")))?;
        let data: EmbedResponse = serde_json::from_value(response).map_err(|e| {
            AppError::InternalError(format!("Failed to parse Ollama response: {e}"))
        })?;
        let embedding = match &data.embeddings[..] {
            [embedding] => embedding.clone(),
            _ => {
                return Err(AppError::InternalError(
                    "Did not receive expected embedding count".to_string(),
                ));
            }
        };
        Ok(embedding)
    }

    async fn generate_stream(&self, system: &str, user: &str) -> Result<TokenStream, AppError> {
        let url = format!("{}/api/generate", self.ollama_url);
        let request = GenerateRequest {
            model: &self.generation_model,
            prompt: user,
            system,
            stream: true,
        };
        let body = serde_json::to_value(request).map_err(|e| {
            AppError::InternalError(format!("Failed to convert ollama request to Value: {e}"))
        })?;

        let headers = vec![("Content-Type", "application/json")];

        let bytes = self
            .client
            .post_stream(&url, body, headers)
            .await
            .map_err(|e| AppError::InternalError(format!("Ollama client failure: {e}")))?;

        let token_stream = stream! {
            let mut bytes = bytes;
            let mut buffer = Vec::new();
            let mut emitted_any = false;
            let mut completed = false;
            'chunks: while let Some(chunk) = bytes.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        error!(error = %e, "AI byte stream error");
                        yield Err(AppError::InternalError("Generation stream error".to_string()));
                        return;
                    }
                };
                buffer.extend_from_slice(&chunk);
                while let Some(idx) = buffer.iter().position(|b| *b == b'\n') {
                    let line = buffer[..idx].trim_ascii_whitespace().to_vec();
                    buffer.drain(..idx + 1);

                    if line.is_empty() {
                        continue;
                    }

                    let parsed = match parse_generate_line(&line) {
                        Ok(parsed) => parsed,
                        Err(e) => {
                            yield Err(e);
                            return;
                        }
                    };

                    if let Some(token) = parsed.response && !token.is_empty() {
                        emitted_any = true;
                        yield Ok(token);
                    }
                    if parsed.done {
                        completed = true;
                        break;
                    }
                }

                if completed {
                    break 'chunks;
                }
            }

            let line = buffer.trim_ascii_whitespace();
            if !line.is_empty() {
                let parsed = match parse_generate_line(line) {
                    Ok(parsed) => parsed,
                    Err(e) => {
                        yield Err(e);
                        return;
                    }
                };
                if let Some(token) = parsed.response && !token.is_empty() {
                    emitted_any = true;
                    yield Ok(token);
                }
                completed = parsed.done;
            }

            if !completed {
                yield Err(AppError::InternalError(
                    "Generation stream ended before Ollama reported completion".to_string(),
                ));
                return;
            }

            if !emitted_any {
                yield Err(AppError::InternalError(
                    "Generation produced no tokens".to_string(),
                ));
                return;
            }
        };

        Ok(Box::pin(token_stream))
    }
}

trait TrimAsciiWhitespace {
    fn trim_ascii_whitespace(&self) -> &[u8];
}

impl TrimAsciiWhitespace for [u8] {
    fn trim_ascii_whitespace(&self) -> &[u8] {
        let start = self
            .iter()
            .position(|b| !b.is_ascii_whitespace())
            .unwrap_or(self.len());
        let end = self
            .iter()
            .rposition(|b| !b.is_ascii_whitespace())
            .map(|idx| idx + 1)
            .unwrap_or(start);
        &self[start..end]
    }
}
