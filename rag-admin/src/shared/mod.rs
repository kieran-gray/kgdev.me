use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostSummary {
    pub slug: String,
    pub title: String,
    pub content_hash: String,
    pub published_at: String,
    pub manifest_post_version: Option<String>,
    pub manifest_chunk_count: Option<u32>,
    pub manifest_ingested_at: Option<String>,
    pub is_dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostDetailDto {
    pub slug: String,
    pub title: String,
    pub published_at: String,
    pub current_post_version: String,
    pub manifest_post_version: Option<String>,
    pub manifest_chunk_count: Option<u32>,
    pub manifest_ingested_at: Option<String>,
    pub is_dirty: bool,
    pub markdown_body_length: u32,
    pub plain_text_excerpt: String,
    pub glossary_terms: Vec<GlossaryTermDto>,
    pub chunk_preview: Vec<ChunkPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryTermDto {
    pub slug: String,
    pub term: String,
    pub definition_excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkPreview {
    pub chunk_id: u32,
    pub heading: String,
    pub text_excerpt: String,
    pub text_length: u32,
    pub is_glossary: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Success,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct IngestOptions {
    pub force: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestJobInfo {
    pub job_id: String,
    pub stream_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SettingsDto {
    pub blog_url: String,
    pub vectorize_index_name: String,
    pub embedding_model: String,
    pub cloudflare_account_id: String,
    pub cloudflare_api_token: String,
    pub kv_namespace_id: String,
}
