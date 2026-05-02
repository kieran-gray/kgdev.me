use async_trait::async_trait;

use crate::server::application::AppError;
use crate::server::domain::{BlogPost, BlogPostSummary};

#[async_trait]
pub trait BlogSource: Send + Sync {
    async fn list(&self) -> Result<Vec<BlogPostSummary>, AppError>;
    async fn fetch(&self, slug: &str) -> Result<BlogPost, AppError>;
}
