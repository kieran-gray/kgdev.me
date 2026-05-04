use serde::{Deserialize, Serialize};

use super::chunking::ChunkingConfig;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostSummary {
    pub slug: String,
    pub title: String,
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
    pub embedding_token_limit: u32,
    pub effective_chunking: ChunkingConfig,
    pub default_chunking: ChunkingConfig,
    pub glossary_terms: Vec<GlossaryTermDto>,
    pub chunk_preview: Vec<ChunkPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryTermDto {
    pub slug: String,
    pub term: String,
    pub definition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkPreview {
    pub chunk_id: u32,
    pub heading: String,
    pub text_excerpt: String,
    pub tokens: Vec<String>,
    pub token_count: u32,
    pub text_length: u32,
    pub is_glossary: bool,
}
