use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Method;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::server::application::ingest::ports::vector_index::{
    MetadataFilterOperation, VectorIndexDescription, VectorMatch, VectorQuery,
};
use crate::server::application::ingest::ports::VectorIndex;
use crate::server::application::AppError;
use crate::server::domain::configuration::ConfigurationRepository;
use crate::server::domain::pipeline_configuration::PipelineConfigurationRepository;
use crate::server::domain::VectorRecord;
use crate::server::infrastructure::clients::{CloudflareApi, CLOUDFLARE_API_BASE};

pub struct VectorizeVectorIndex {
    api: Arc<CloudflareApi>,
    configuration: Arc<dyn ConfigurationRepository>,
    pipeline_configuration: Arc<dyn PipelineConfigurationRepository>,
}

enum VectorizeOperation {
    Upsert,
    Delete,
    Query,
    Describe,
}

impl VectorizeOperation {
    fn as_api_path(&self) -> &str {
        match self {
            Self::Upsert => "upsert",
            Self::Delete => "delete-by-ids",
            Self::Query => "query",
            Self::Describe => "info",
        }
    }

    fn as_label(&self) -> &str {
        match self {
            Self::Upsert => "vectorize-upsert",
            Self::Delete => "vectorize-delete",
            Self::Query => "vectorize-query",
            Self::Describe => "vectorize-describe",
        }
    }
}

#[derive(Deserialize)]
struct QueryEnvelope {
    result: Option<QueryResult>,
}

#[derive(Deserialize)]
struct QueryResult {
    matches: Vec<QueryMatch>,
}

#[derive(Deserialize)]
struct QueryMatch {
    id: String,
    score: f32,
    metadata: Option<Value>,
}

#[derive(Deserialize)]
struct DescribeEnvelope {
    result: Option<DescribeResult>,
}

#[derive(Deserialize)]
struct DescribeResult {
    name: String,
    config: DescribeConfig,
}

#[derive(Deserialize)]
struct DescribeConfig {
    dimensions: u32,
}

impl VectorizeVectorIndex {
    pub fn new(
        api: Arc<CloudflareApi>,
        configuration: Arc<dyn ConfigurationRepository>,
        pipeline_configuration: Arc<dyn PipelineConfigurationRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            api,
            configuration,
            pipeline_configuration,
        })
    }

    async fn url(&self, operation: &VectorizeOperation) -> Result<String, AppError> {
        let pipeline_configs = self
            .pipeline_configuration
            .load_all()
            .await
            .map_err(|e| AppError::Internal(format!("load pipeline configurations: {e}")))?;
        let vector_index_id = pipeline_configs
            .first()
            .map(|pc| pc.vector_index_id)
            .ok_or_else(|| AppError::Validation("no pipeline configuration is set".into()))?;

        let catalog = self
            .configuration
            .load()
            .await
            .map_err(|e| AppError::Internal(format!("load configuration: {e}")))?;
        let index_name = catalog
            .vector_indexes
            .iter()
            .find(|i| i.index_id == vector_index_id)
            .map(|i| i.name.clone())
            .ok_or_else(|| {
                AppError::Internal(format!(
                    "vector index {vector_index_id} not found in configuration"
                ))
            })?;

        Ok(format!(
            "{}/accounts/{}/vectorize/v2/indexes/{}/{}",
            CLOUDFLARE_API_BASE,
            self.api.account_id(),
            index_name,
            operation.as_api_path()
        ))
    }
}

#[async_trait]
impl VectorIndex for VectorizeVectorIndex {
    async fn upsert(&self, records: &[VectorRecord]) -> Result<(), AppError> {
        if records.is_empty() {
            return Ok(());
        }
        let operation = VectorizeOperation::Upsert;
        let url = self.url(&operation).await?;

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
                operation.as_label(),
            )
            .await?;
        Ok(())
    }

    async fn delete(&self, ids: &[String]) -> Result<(), AppError> {
        if ids.is_empty() {
            return Ok(());
        }
        let operation = VectorizeOperation::Delete;
        let url = self.url(&operation).await?;

        let body = json!({ "ids": ids });
        let body_bytes = serde_json::to_vec(&body)
            .map_err(|e| AppError::Internal(format!("encode delete body: {e}")))?;
        self.api
            .request(
                Method::POST,
                &url,
                body_bytes,
                "application/json",
                operation.as_label(),
            )
            .await?;
        Ok(())
    }

    async fn query(&self, q: &VectorQuery) -> Result<Vec<VectorMatch>, AppError> {
        let operation = VectorizeOperation::Query;
        let url = self.url(&operation).await?;

        let body = json!({
            "vector": q.vector,
            "topK": q.top_k,
            "returnMetadata": "all",
            "returnValues": false,
            "filter": metadata_filter_as_map(q),
        });
        let body_bytes = serde_json::to_vec(&body)
            .map_err(|e| AppError::Internal(format!("encode query body: {e}")))?;
        let response = self
            .api
            .request(
                Method::POST,
                &url,
                body_bytes,
                "application/json",
                operation.as_label(),
            )
            .await?;

        let envelope: QueryEnvelope = serde_json::from_str(&response)
            .map_err(|e| AppError::Internal(format!("vectorize-query response parse: {e}")))?;

        let matches = envelope
            .result
            .map(|r| r.matches)
            .unwrap_or_default()
            .into_iter()
            .map(|m| VectorMatch {
                id: m.id,
                score: m.score,
                metadata: m.metadata.unwrap_or(Value::Null),
            })
            .collect();

        Ok(matches)
    }

    async fn describe(&self) -> Result<VectorIndexDescription, AppError> {
        let operation = VectorizeOperation::Describe;
        let url = self.url(&operation).await?;

        let response = self
            .api
            .request(
                Method::GET,
                &url,
                vec![],
                "application/json",
                operation.as_label(),
            )
            .await?;

        let envelope: DescribeEnvelope = serde_json::from_str(&response)
            .map_err(|e| AppError::Internal(format!("vectorize-describe response parse: {e}")))?;

        let result = envelope.result.ok_or_else(|| {
            AppError::Internal("vectorize-describe: missing result in response".into())
        })?;

        Ok(VectorIndexDescription {
            name: result.name,
            dimensions: result.config.dimensions,
        })
    }
}

fn metadata_filter_operation_as_str(operation: &MetadataFilterOperation) -> &'static str {
    match operation {
        MetadataFilterOperation::Equal => "$eq",
        MetadataFilterOperation::NotEqual => "$ne",
    }
}

fn metadata_filter_as_map(query: &VectorQuery) -> Value {
    let mut filter_map = Map::with_capacity(query.filter.len());

    for item in &query.filter {
        let op = metadata_filter_operation_as_str(&item.operation);
        let entry = json!({ op: item.value });
        filter_map.insert(item.field.clone(), entry);
    }

    Value::Object(filter_map)
}
