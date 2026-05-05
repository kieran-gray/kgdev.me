use serde::{Deserialize, Serialize};

use super::ChunkingConfig;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationGenerationBackend {
    #[default]
    Ollama,
    WorkersAi,
}

impl EvaluationGenerationBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::WorkersAi => "workers_ai",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvaluationSettings {
    #[serde(default)]
    pub generation_backend: EvaluationGenerationBackend,
    #[serde(default = "default_ollama_base_url")]
    pub ollama_base_url: String,
    #[serde(default = "default_generation_model")]
    pub generation_model: String,
    #[serde(
        default = "default_question_count",
        deserialize_with = "crate::shared::serde_compat::u32_from_string"
    )]
    pub question_count: u32,
    #[serde(
        default = "default_excerpt_similarity_threshold_milli",
        deserialize_with = "crate::shared::serde_compat::u32_from_string"
    )]
    pub excerpt_similarity_threshold_milli: u32,
    #[serde(
        default = "default_duplicate_similarity_threshold_milli",
        deserialize_with = "crate::shared::serde_compat::u32_from_string"
    )]
    pub duplicate_similarity_threshold_milli: u32,
    #[serde(
        default = "default_top_k",
        deserialize_with = "crate::shared::serde_compat::u32_from_string"
    )]
    pub top_k: u32,
    #[serde(
        default,
        deserialize_with = "crate::shared::serde_compat::u32_from_string"
    )]
    pub min_score_milli: u32,
    #[serde(default = "default_include_glossary")]
    pub include_glossary: bool,
}

impl Default for EvaluationSettings {
    fn default() -> Self {
        Self {
            generation_backend: EvaluationGenerationBackend::Ollama,
            ollama_base_url: default_ollama_base_url(),
            generation_model: default_generation_model(),
            question_count: default_question_count(),
            excerpt_similarity_threshold_milli: default_excerpt_similarity_threshold_milli(),
            duplicate_similarity_threshold_milli: default_duplicate_similarity_threshold_milli(),
            top_k: default_top_k(),
            min_score_milli: 0,
            include_glossary: default_include_glossary(),
        }
    }
}

impl EvaluationSettings {
    pub fn excerpt_similarity_threshold(&self) -> f32 {
        milli_to_f32(self.excerpt_similarity_threshold_milli)
    }

    pub fn duplicate_similarity_threshold(&self) -> f32 {
        milli_to_f32(self.duplicate_similarity_threshold_milli)
    }

    pub fn min_score(&self) -> f32 {
        milli_to_f32(self.min_score_milli)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvaluationReference {
    pub content: String,
    #[serde(deserialize_with = "crate::shared::serde_compat::u32_from_string")]
    pub char_start: u32,
    #[serde(deserialize_with = "crate::shared::serde_compat::u32_from_string")]
    pub char_end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvaluationQuestion {
    pub question: String,
    pub references: Vec<EvaluationReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvaluationDataset {
    pub slug: String,
    pub post_version: String,
    pub generated_at: String,
    pub questions: Vec<EvaluationQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvaluationDatasetStatus {
    pub slug: String,
    pub post_version: String,
    pub exists: bool,
    pub question_count: u32,
    pub generated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationJobInfo {
    pub job_id: String,
    pub stream_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvaluationRunOptions {
    #[serde(deserialize_with = "crate::shared::serde_compat::u32_from_string")]
    pub top_k: u32,
    #[serde(deserialize_with = "crate::shared::serde_compat::u32_from_string")]
    pub min_score_milli: u32,
    pub include_glossary: bool,
}

impl Default for EvaluationRunOptions {
    fn default() -> Self {
        Self {
            top_k: default_top_k(),
            min_score_milli: 0,
            include_glossary: default_include_glossary(),
        }
    }
}

impl EvaluationRunOptions {
    pub fn min_score(&self) -> f32 {
        milli_to_f32(self.min_score_milli)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkingVariant {
    pub label: String,
    pub config: ChunkingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationMetrics {
    pub recall_mean: f32,
    pub recall_std: f32,
    pub precision_mean: f32,
    pub precision_std: f32,
    pub iou_mean: f32,
    pub iou_std: f32,
    pub precision_omega_mean: f32,
    pub precision_omega_std: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationQuestionResult {
    pub question: String,
    pub recall: f32,
    pub precision: f32,
    pub iou: f32,
    pub retrieved_chunk_ids: Vec<u32>,
    pub missed_reference_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationVariantResult {
    pub variant: ChunkingVariant,
    pub metrics: EvaluationMetrics,
    pub chunk_count: u32,
    pub average_chunk_chars: u32,
    pub average_retrieved_chars: u32,
    pub question_results: Vec<EvaluationQuestionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationRunResult {
    pub slug: String,
    pub post_version: String,
    pub options: EvaluationRunOptions,
    pub variants: Vec<EvaluationVariantResult>,
}

fn milli_to_f32(value: u32) -> f32 {
    value as f32 / 1000.0
}

fn default_ollama_base_url() -> String {
    "http://localhost:11434".into()
}

fn default_generation_model() -> String {
    "granite4.1:8b".into()
}

fn default_question_count() -> u32 {
    8
}

fn default_excerpt_similarity_threshold_milli() -> u32 {
    360
}

fn default_duplicate_similarity_threshold_milli() -> u32 {
    700
}

fn default_top_k() -> u32 {
    5
}

fn default_include_glossary() -> bool {
    true
}
