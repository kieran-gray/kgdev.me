use async_trait::async_trait;

use crate::api_worker::application::AppError;

#[async_trait(?Send)]
pub trait AiInferenceServiceTrait {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AppError>;
    async fn generate(&self, system: &str, user: &str) -> Result<String, AppError>;
}
