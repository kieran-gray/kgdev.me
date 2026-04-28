use crate::application::exceptions::AppError;
use async_trait::async_trait;

#[async_trait(?Send)]
pub trait RequestValidationServiceTrait: Send + Sync {
    async fn verify(&self, token: String, ip: String) -> Result<(), AppError>;
}
