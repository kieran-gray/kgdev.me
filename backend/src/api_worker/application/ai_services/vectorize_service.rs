use async_trait::async_trait;

use crate::api_worker::application::AppError;

#[derive(Debug, Clone)]
pub struct ScoredChunk {
    pub chunk_id: u32,
    pub heading: String,
    pub text: String,
    pub post_slug: String,
    pub score: f32,
}

#[async_trait(?Send)]
pub trait VectorizeServiceTrait {
    async fn query(
        &self,
        embedding: &[f32],
        post_slug: &str,
        post_version: &str,
        top_k: u32,
    ) -> Result<Vec<ScoredChunk>, AppError>;
}
