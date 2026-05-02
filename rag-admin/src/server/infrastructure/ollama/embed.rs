use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Method;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::RwLock;

use crate::server::application::ports::Embedder;
use crate::server::application::AppError;
use crate::server::infrastructure::ollama::client::{OllamaApi, API_BASE};
use crate::shared::SettingsDto;

pub struct OllamaEmbedder {
    api: Arc<OllamaApi>,
    settings: Arc<RwLock<SettingsDto>>,
}

impl OllamaEmbedder {
    pub fn new(api: Arc<OllamaApi>, settings: Arc<RwLock<SettingsDto>>) -> Arc<Self> {
        Arc::new(Self { api, settings })
    }
}

#[derive(Debug, Deserialize)]
struct EmbedResult {
    embeddings: Vec<Vec<f32>>,
}

#[async_trait]
impl Embedder for OllamaEmbedder {
    async fn embed_batch(&self, model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, AppError> {
        let dims = self.settings.read().await.embed_dimensions;
        let url = format!("{}/api/embed", API_BASE);
        let body = json!({ "model": model, "input": texts, "dimensions": dims });
        let body_bytes = serde_json::to_vec(&body)
            .map_err(|e| AppError::Internal(format!("encode embed body: {e}")))?;
        let resp = self
            .api
            .request(
                Method::POST,
                &url,
                body_bytes,
                "application/json",
                "embedding",
            )
            .await?;
        let data: EmbedResult = serde_json::from_str(&resp)
            .map_err(|e| AppError::Upstream(format!("parse embedding: {e}")))?;
        if data.embeddings.len() != texts.len() {
            return Err(AppError::Upstream(format!(
                "ollama returned {} embeddings for {} inputs",
                data.embeddings.len(),
                texts.len()
            )));
        }
        Ok(data.embeddings)
    }
}
