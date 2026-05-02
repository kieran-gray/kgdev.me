use std::path::{Path, PathBuf};

use tokio::fs;

use crate::server::setup::exceptions::SetupError;
use crate::shared::{ChunkStrategy, SettingsDto};

pub fn data_dir() -> PathBuf {
    std::env::current_dir()
        .map(|p| p.join("rag-admin").join("data"))
        .unwrap_or_else(|_| PathBuf::from("rag-admin/data"))
}

pub fn settings_path() -> PathBuf {
    data_dir().join("settings.toml")
}

pub fn manifest_path() -> PathBuf {
    data_dir().join("manifest.json")
}

pub fn tokenizer_path() -> PathBuf {
    data_dir().join("tokenizer.json")
}

pub fn defaults() -> SettingsDto {
    SettingsDto {
        blog_url: "http://localhost:4321".into(),
        vectorize_index_name: "blog-chunks".into(),
        embedding_model: "@cf/qwen/qwen3-embedding-0.6b".into(),
        cloudflare_account_id: String::new(),
        cloudflare_api_token: String::new(),
        kv_namespace_id: String::new(),
        embedder_backend: "cloudflare".into(),
        embed_dimensions: 1024,
        chunk_strategy: ChunkStrategy::Section,
    }
}

pub async fn load_settings(path: &Path) -> Result<SettingsDto, SetupError> {
    let mut settings = defaults();
    if path.exists() {
        let bytes = fs::read(path)
            .await
            .map_err(|e| SetupError::Io(format!("read settings: {e}")))?;
        if !bytes.is_empty() {
            let text = std::str::from_utf8(&bytes)
                .map_err(|e| SetupError::Config(format!("settings.toml not utf-8: {e}")))?;
            let parsed: SettingsDto = toml::from_str(text)
                .map_err(|e| SetupError::Config(format!("parse settings.toml: {e}")))?;
            settings = parsed;
        }
    }
    overlay_env(&mut settings);
    Ok(settings)
}

pub async fn save_settings(path: &Path, settings: &SettingsDto) -> Result<(), SetupError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| SetupError::Io(format!("create settings dir: {e}")))?;
    }
    let s = toml::to_string_pretty(settings)
        .map_err(|e| SetupError::Internal(format!("encode settings: {e}")))?;
    fs::write(path, s)
        .await
        .map_err(|e| SetupError::Io(format!("write settings: {e}")))
}

fn overlay_env(s: &mut SettingsDto) {
    if let Ok(v) = std::env::var("BLOG_URL") {
        if !v.is_empty() {
            s.blog_url = v;
        }
    }
    if let Ok(v) = std::env::var("VECTORIZE_INDEX_NAME") {
        if !v.is_empty() {
            s.vectorize_index_name = v;
        }
    }
    if let Ok(v) = std::env::var("EMBEDDING_MODEL") {
        if !v.is_empty() {
            s.embedding_model = v;
        }
    }
    if let Ok(v) = std::env::var("CLOUDFLARE_ACCOUNT_ID") {
        if !v.is_empty() {
            s.cloudflare_account_id = v;
        }
    }
    if let Ok(v) = std::env::var("CLOUDFLARE_RAG_INGEST_API_TOKEN") {
        if !v.is_empty() {
            s.cloudflare_api_token = v;
        }
    }
    if let Ok(v) = std::env::var("BLOG_POST_QA_CACHE_KV_NAMESPACE_ID") {
        if !v.is_empty() {
            s.kv_namespace_id = v;
        }
    }
}
