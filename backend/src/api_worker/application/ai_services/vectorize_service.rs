use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::api_worker::application::AppError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Reference {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct ScoredChunk {
    pub chunk_id: u32,
    pub heading: String,
    pub text: String,
    pub source_slug: String,
    pub score: f32,
    pub references: Vec<Reference>,
}

#[derive(Debug, Clone, Copy)]
pub struct QueryFilter<'a> {
    pub source_slug: &'a str,
    pub source_version: &'a str,
}

#[async_trait(?Send)]
pub trait VectorizeServiceTrait {
    async fn query(
        &self,
        embedding: &[f32],
        filter: Option<QueryFilter<'_>>,
        top_k: u32,
        min_score: f32,
    ) -> Result<Vec<ScoredChunk>, AppError>;
}
