use async_trait::async_trait;

use crate::server::application::{IngestLogEvent, Job};

#[async_trait]
pub trait EvaluationProgress: Send + Sync {
    async fn info(&self, message: String);
    async fn warn(&self, message: String);
    async fn error(&self, message: String);
    async fn success(&self, message: String);
}

#[async_trait]
impl EvaluationProgress for Job {
    async fn info(&self, message: String) {
        self.emit(IngestLogEvent::info(message)).await
    }
    async fn warn(&self, message: String) {
        self.emit(IngestLogEvent::warn(message)).await
    }
    async fn error(&self, message: String) {
        self.emit(IngestLogEvent::error(message)).await
    }
    async fn success(&self, message: String) {
        self.emit(IngestLogEvent::success(message)).await
    }
}
