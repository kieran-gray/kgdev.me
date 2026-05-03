use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::server::application::ports::{Embedder, VectorStore};
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
use crate::server::setup::validation;
use crate::shared::{EmbedderBackend, SettingsDto};

pub struct AppState {
    pub settings: Arc<RwLock<SettingsDto>>,
    pub ingest_service: Arc<IngestService>,
    pub job_registry: Arc<JobRegistry>,
    pub vector_store: Arc<dyn VectorStore>,
    pub embedder: Arc<dyn Embedder>,
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

        let embedder: Arc<dyn Embedder> = Arc::new(BackendEmbedder {
            cloudflare: WorkersAiEmbedder::new(cf_api.clone()),
            ollama: OllamaEmbedder::new(ollama_api, settings.clone()),
            settings: settings.clone(),
        });

        let vector_store: Arc<dyn VectorStore> = CloudflareVectorStore::new(cf_api.clone());
        let kv_store = CloudflareKvStore::new(cf_api.clone());

        let manifest_store = FileManifestStore::new(manifest_path());
        let tokenizer = HuggingFaceTokenizer::load_or_fetch(tokenizer_path(), http.clone())
            .await
            .map_err(|e| SetupError::Internal(format!("tokenizer: {e}")))?;
        let job_registry = Arc::new(JobRegistry::new());

        let ingest_service = IngestService::new(
            blog_source,
            embedder.clone(),
            vector_store.clone(),
            kv_store,
            manifest_store,
            tokenizer,
            EMBEDDING_TOKEN_LIMIT,
            settings.clone(),
            job_registry.clone(),
        );

        let state = Self {
            settings,
            ingest_service,
            job_registry,
            vector_store,
            embedder,
        };

        if let Err(e) = state.validate_active_settings().await {
            tracing::warn!("settings invariant check: {e}");
        }

        Ok(state)
    }

    pub async fn settings_snapshot(&self) -> SettingsDto {
        self.settings.read().await.clone()
    }

    pub async fn save_settings(&self, new_settings: SettingsDto) -> Result<(), SetupError> {
        validation::validate_local(&new_settings).map_err(SetupError::Config)?;
        let previous = {
            let mut guard = self.settings.write().await;
            std::mem::replace(&mut *guard, new_settings.clone())
        };
        if let Err(e) = save_settings(&settings_path(), &new_settings).await {
            *self.settings.write().await = previous;
            return Err(e);
        }
        Ok(())
    }

    async fn validate_active_settings(&self) -> Result<(), String> {
        let snapshot = self.settings_snapshot().await;
        validation::validate_local(&snapshot)
    }
}

struct BackendEmbedder {
    cloudflare: Arc<dyn Embedder>,
    ollama: Arc<dyn Embedder>,
    settings: Arc<RwLock<SettingsDto>>,
}

#[async_trait]
impl Embedder for BackendEmbedder {
    async fn embed_batch(&self, model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, AppError> {
        let backend = self.settings.read().await.embedding_model.backend;
        match backend {
            EmbedderBackend::Cloudflare => self.cloudflare.embed_batch(model, texts).await,
            EmbedderBackend::Ollama => self.ollama.embed_batch(model, texts).await,
        }
    }
}
