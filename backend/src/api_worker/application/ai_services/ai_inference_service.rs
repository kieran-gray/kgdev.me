use std::pin::Pin;

use async_trait::async_trait;
use futures_util::Stream;

use crate::api_worker::application::AppError;

pub type TokenStream = Pin<Box<dyn Stream<Item = Result<String, AppError>>>>;

#[async_trait(?Send)]
pub trait AiInferenceServiceTrait {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AppError>;
    async fn generate_stream(&self, system: &str, user: &str) -> Result<TokenStream, AppError>;
}
