use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{error, warn};

use crate::api_worker::{
    application::{AppError, ScoredChunk, VectorizeServiceTrait},
    infrastructure::http_client::HttpClientTrait,
};

pub struct VectorizeRestService {
    cloudflare_account_id: String,
    cloudflare_api_token: String,
    index_name: String,
    http_client: Arc<dyn HttpClientTrait>,
}

impl VectorizeRestService {
    pub fn create(
        cloudflare_account_id: String,
        cloudflare_api_token: String,
        index_name: String,
        http_client: Arc<dyn HttpClientTrait>,
    ) -> Arc<Self> {
        Arc::new(Self {
            cloudflare_account_id,
            cloudflare_api_token,
            index_name,
            http_client,
        })
    }
}

#[derive(Deserialize)]
struct QueryEnvelope {
    success: Option<bool>,
    result: Option<QueryResult>,
}

#[derive(Deserialize)]
struct QueryResult {
    matches: Vec<QueryMatch>,
}

#[derive(Deserialize)]
struct QueryMatch {
    score: f32,
    metadata: Option<MatchMetadata>,
}

#[derive(Deserialize)]
struct MatchMetadata {
    chunk_id: u32,
    heading: Option<String>,
    text: String,
    post_slug: String,
}

#[async_trait(?Send)]
impl VectorizeServiceTrait for VectorizeRestService {
    async fn query(
        &self,
        embedding: &[f32],
        post_slug: &str,
        top_k: u32,
    ) -> Result<Vec<ScoredChunk>, AppError> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/vectorize/v2/indexes/{}/query",
            self.cloudflare_account_id, self.index_name
        );
        let token = format!("Bearer {}", self.cloudflare_api_token);
        let headers = vec![
            ("Authorization", token.as_str()),
            ("Content-Type", "application/json"),
        ];
        let body = json!({
            "vector": embedding,
            "topK": top_k,
            "returnMetadata": "all",
            "returnValues": false,
        });

        let raw: Value = self
            .http_client
            .post(&url, body, headers)
            .await
            .map_err(|e| {
                error!(error = %e, "Vectorize query failed");
                AppError::InternalError(format!("Vectorize query failed: {e}"))
            })?;

        let envelope: QueryEnvelope = serde_json::from_value(raw.clone()).map_err(|e| {
            error!(error = %e, body = %raw, "Vectorize response could not be parsed");
            AppError::InternalError("Vectorize response was malformed".to_string())
        })?;

        if !envelope.success.unwrap_or(false) {
            warn!(body = %raw, "Vectorize returned non-success");
            return Err(AppError::InternalError(
                "Vectorize returned non-success".to_string(),
            ));
        }

        let matches = envelope
            .result
            .map(|r| r.matches)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|m| {
                let metadata = m.metadata?;
                if metadata.post_slug != post_slug {
                    return None;
                }
                Some(ScoredChunk {
                    chunk_id: metadata.chunk_id,
                    heading: metadata.heading.unwrap_or_default(),
                    text: metadata.text,
                    post_slug: metadata.post_slug,
                    score: m.score,
                })
            })
            .collect();

        Ok(matches)
    }
}
