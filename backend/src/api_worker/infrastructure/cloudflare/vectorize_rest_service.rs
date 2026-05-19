use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tracing::{error, warn};

use crate::api_worker::{
    application::{AppError, QueryFilter, Reference, ScoredChunk, VectorizeServiceTrait},
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
    source_slug: String,
    source_version: Option<String>,
    #[serde(default)]
    reference_titles: Vec<String>,
    #[serde(default)]
    reference_urls: Vec<String>,
}

#[async_trait(?Send)]
impl VectorizeServiceTrait for VectorizeRestService {
    async fn query(
        &self,
        embedding: &[f32],
        filter: Option<QueryFilter<'_>>,
        top_k: u32,
        min_score: f32,
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
        let mut body = json!({
            "vector": embedding,
            "topK": top_k,
            "returnMetadata": "all",
            "returnValues": false,
        });
        if let Some(QueryFilter {
            source_slug,
            source_version,
        }) = filter
            && let Some(obj) = body.as_object_mut()
        {
            obj.insert(
                "filter".to_string(),
                json!({
                    "source_slug": { "$eq": source_slug },
                    "source_version": { "$eq": source_version },
                }),
            );
        }

        let response_json = self
            .http_client
            .post(&url, body, headers)
            .await
            .map_err(|e| {
                error!(error = %e, "Vectorize query failed");
                AppError::InternalError(format!("Vectorize query failed: {e}"))
            })?;

        let envelope: QueryEnvelope =
            serde_json::from_value(response_json.clone()).map_err(|e| {
                error!(error = %e, body = %response_json, "Vectorize response could not be parsed");
                AppError::InternalError("Vectorize response was malformed".to_string())
            })?;

        if !envelope.success.unwrap_or(false) {
            warn!(body = %response_json, "Vectorize returned non-success");
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
                if let Some(QueryFilter {
                    source_slug,
                    source_version,
                }) = filter
                {
                    if metadata.source_slug != source_slug {
                        return None;
                    }
                    if metadata.source_version.as_deref() != Some(source_version) {
                        return None;
                    }
                }
                if m.score < min_score {
                    return None;
                }
                let references = metadata
                    .reference_titles
                    .into_iter()
                    .zip(metadata.reference_urls)
                    .map(|(title, url)| Reference { title, url })
                    .collect();
                Some(ScoredChunk {
                    chunk_id: metadata.chunk_id,
                    heading: metadata.heading.unwrap_or_default(),
                    text: metadata.text,
                    source_slug: metadata.source_slug,
                    score: m.score,
                    references,
                })
            })
            .collect();

        Ok(matches)
    }
}
