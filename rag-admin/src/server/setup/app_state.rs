use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::server::application::ports::Embedder;
use crate::server::application::{AppError, IngestService, JobRegistry};
use crate::server::infrastructure::cloudflare::client::CloudflareApi;
use crate::server::infrastructure::ollama::client::OllamaApi;
use crate::server::infrastructure::{
    CloudflareKvStore, CloudflareVectorStore, FileManifestStore, HttpBlogSource,
    HuggingFaceTokenizer, OllamaEmbedder, ReqwestHttpClient, WorkersAiEmbedder,
    EMBEDDING_TOKEN_LIMIT,
};
use crate::server::setup::config::{
    load_settings, manifest_path, save_settings, settings_path, tokenizer_path,
};
use crate::server::setup::exceptions::SetupError;
use crate::shared::SettingsDto;

pub struct AppState {
    pub settings: Arc<RwLock<SettingsDto>>,
    pub ingest_service: Arc<IngestService>,
    pub job_registry: Arc<JobRegistry>,
}

impl AppState {
    pub async fn initialize() -> Result<Self, SetupError> {
        let settings = load_settings(&settings_path()).await?;
        let settings = Arc::new(RwLock::new(settings));

        let http = Arc::new(
            ReqwestHttpClient::new()
                .map_err(|e| SetupError::Internal(format!("http client: {e}")))?,
        );

        let blog_source = HttpBlogSource::new(http.clone(), settings.clone());

        let cf_api = Arc::new(CloudflareApi::new(http.clone(), settings.clone()));
        let ollama_api = Arc::new(OllamaApi::new(http.clone()));

        let embedder: Arc<dyn Embedder> = Arc::new(DynEmbedder {
            ollama: OllamaEmbedder::new(ollama_api, settings.clone()),
            cloudflare: WorkersAiEmbedder::new(cf_api.clone()),
            settings: settings.clone(),
        });

        let vector_store = CloudflareVectorStore::new(cf_api.clone());
        let kv_store = CloudflareKvStore::new(cf_api.clone());

        let manifest_store = FileManifestStore::new(manifest_path());
        let tokenizer = HuggingFaceTokenizer::load_or_fetch(tokenizer_path(), http.clone())
            .await
            .map_err(|e| SetupError::Internal(format!("tokenizer: {e}")))?;
        let job_registry = Arc::new(JobRegistry::new());

        let ingest_service = IngestService::new(
            blog_source,
            embedder,
            vector_store,
            kv_store,
            manifest_store,
            tokenizer,
            EMBEDDING_TOKEN_LIMIT,
            settings.clone(),
            job_registry.clone(),
        );

        Ok(Self {
            settings,
            ingest_service,
            job_registry,
        })
    }

    pub async fn settings_snapshot(&self) -> SettingsDto {
        self.settings.read().await.clone()
    }

    pub async fn save_settings(&self, new_settings: SettingsDto) -> Result<(), SetupError> {
        save_settings(&settings_path(), &new_settings).await?;
        *self.settings.write().await = new_settings;
        Ok(())
    }
}

struct DynEmbedder {
    ollama: Arc<dyn Embedder>,
    cloudflare: Arc<dyn Embedder>,
    settings: Arc<RwLock<SettingsDto>>,
}

#[async_trait]
impl Embedder for DynEmbedder {
    async fn embed_batch(&self, model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, AppError> {
        if self.settings.read().await.embedder_backend == "ollama" {
            self.ollama.embed_batch(model, texts).await
        } else {
            self.cloudflare.embed_batch(model, texts).await
        }
    }
}
