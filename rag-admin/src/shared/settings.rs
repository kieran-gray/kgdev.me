use serde::{Deserialize, Serialize};

use super::chunking::ChunkingConfig;
use super::embedding::EmbeddingModel;
use super::evaluation::EvaluationSettings;
use super::vector::VectorIndexConfig;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SettingsDto {
    pub blog_url: String,
    pub cloudflare_account_id: String,
    pub cloudflare_api_token: String,
    pub kv_namespace_id: String,
    #[serde(default)]
    pub vector_index: VectorIndexConfig,
    #[serde(default)]
    pub embedding_model: EmbeddingModel,
    #[serde(default)]
    pub default_chunking: ChunkingConfig,
    #[serde(default)]
    pub evaluation: EvaluationSettings,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deserializes_numeric_strings_from_server_function_payloads() {
        let parsed: SettingsDto = serde_json::from_value(json!({
            "blog_url": "https://example.com",
            "cloudflare_account_id": "acct",
            "cloudflare_api_token": "token",
            "kv_namespace_id": "kv",
            "vector_index": {
                "provider": "cloudflare",
                "name": "blog-chunks",
                "dimensions": "1024"
            },
            "embedding_model": {
                "backend": "ollama",
                "id": "qwen3-embedding:0.6b",
                "dims": "1024"
            },
            "default_chunking": {
                "strategy": "section",
                "max_section_chars": "8000",
                "target_chars": "1600",
                "overlap_chars": "240",
                "min_chars": "320"
            },
            "evaluation": {
                "generation_backend": "ollama",
                "ollama_base_url": "http://localhost:11434",
                "generation_model": "granite4.1:8b",
                "question_count": "8",
                "excerpt_similarity_threshold_milli": "360",
                "duplicate_similarity_threshold_milli": "700",
                "top_k": "5",
                "min_score_milli": "0",
                "include_glossary": true
            }
        }))
        .unwrap();

        assert_eq!(parsed.vector_index.dimensions(), 1024);
        assert_eq!(parsed.embedding_model.dims, 1024);
        assert_eq!(parsed.default_chunking.max_section_chars, 8000);
        assert_eq!(parsed.evaluation.question_count, 8);
    }
}
