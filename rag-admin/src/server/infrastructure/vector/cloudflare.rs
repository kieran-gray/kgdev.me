use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Method;
use serde_json::json;

use crate::server::application::ingest::ports::VectorStore;
use crate::server::application::AppError;
use crate::server::domain::VectorRecord;
use crate::server::infrastructure::clients::{CloudflareApi, CLOUDFLARE_API_BASE};

pub struct CloudflareVectorStore {
    api: Arc<CloudflareApi>,
}

impl CloudflareVectorStore {
    pub fn new(api: Arc<CloudflareApi>) -> Arc<Self> {
        Arc::new(Self { api })
    }
}

#[async_trait]
impl VectorStore for CloudflareVectorStore {
    async fn upsert(&self, index: &str, records: &[VectorRecord]) -> Result<(), AppError> {
        if records.is_empty() {
            return Ok(());
        }
        let creds = self.api.credentials().await?;
        let url = format!(
            "{}/accounts/{}/vectorize/v2/indexes/{}/upsert",
            CLOUDFLARE_API_BASE, creds.account_id, index
        );
        let mut ndjson = String::new();
        for r in records {
            let line = serde_json::to_string(r)
                .map_err(|e| AppError::Internal(format!("encode vector record: {e}")))?;
            ndjson.push_str(&line);
            ndjson.push('\n');
        }
        self.api
            .request(
                Method::POST,
                &url,
                ndjson.into_bytes(),
                "application/x-ndjson",
                "vectorize-upsert",
            )
            .await?;
        Ok(())
    }

    async fn delete_ids(&self, index: &str, ids: &[String]) -> Result<(), AppError> {
        if ids.is_empty() {
            return Ok(());
        }
        let creds = self.api.credentials().await?;
        let url = format!(
            "{}/accounts/{}/vectorize/v2/indexes/{}/delete-by-ids",
            CLOUDFLARE_API_BASE, creds.account_id, index
        );
        let body = json!({ "ids": ids });
        let body_bytes = serde_json::to_vec(&body)
            .map_err(|e| AppError::Internal(format!("encode delete body: {e}")))?;
        self.api
            .request(
                Method::POST,
                &url,
                body_bytes,
                "application/json",
                "vectorize-delete",
            )
            .await?;
        Ok(())
    }
}
