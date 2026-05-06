use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use crate::server::application::evaluation::ports::EvaluationDatasetStore;
use crate::server::application::AppError;
use crate::shared::{EvaluationDataset, EvaluationDatasetStatus};

pub struct FileEvaluationDatasetStore {
    path: PathBuf,
}

impl FileEvaluationDatasetStore {
    pub fn new(path: PathBuf) -> Arc<Self> {
        Arc::new(Self { path })
    }

    fn dataset_path(&self, slug: &str, version: &str) -> PathBuf {
        self.path
            .join(sanitize_path_segment(slug))
            .join(format!("{version}.json"))
    }

    async fn read_from_disk(
        &self,
        slug: &str,
        version: &str,
    ) -> Result<EvaluationDataset, AppError> {
        let path = self.dataset_path(slug, version);
        let bytes = tokio::fs::read(&path)
            .await
            .map_err(|e| AppError::Io(format!("read evaluation dataset: {e}")))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| AppError::Internal(format!("parse evaluation dataset: {e}")))
    }

    async fn write_to_disk(&self, dataset: &EvaluationDataset) -> Result<(), AppError> {
        let path = self.dataset_path(&dataset.slug, &dataset.post_version);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AppError::Io(format!("create evaluation dataset dir: {e}")))?;
        }
        let bytes = serde_json::to_vec_pretty(dataset)
            .map_err(|e| AppError::Internal(format!("encode evaluation dataset: {e}")))?;
        tokio::fs::write(path, bytes)
            .await
            .map_err(|e| AppError::Io(format!("write evaluation dataset: {e}")))
    }
}

#[async_trait]
impl EvaluationDatasetStore for FileEvaluationDatasetStore {
    async fn load(&self, slug: &str, version: &str) -> Result<EvaluationDataset, AppError> {
        self.read_from_disk(slug, version).await
    }

    async fn status(&self, slug: &str, version: &str) -> Result<EvaluationDatasetStatus, AppError> {
        let path = self.dataset_path(slug, version);
        if !path.exists() {
            return Ok(EvaluationDatasetStatus {
                slug: slug.to_string(),
                post_version: version.to_string(),
                exists: false,
                question_count: 0,
                generated_at: None,
            });
        }
        let dataset = self.load(slug, version).await?;
        Ok(EvaluationDatasetStatus {
            slug: slug.to_string(),
            post_version: version.to_string(),
            exists: true,
            question_count: dataset.questions.len() as u32,
            generated_at: Some(dataset.generated_at),
        })
    }

    async fn store(&self, dataset: &EvaluationDataset) -> Result<(), AppError> {
        self.write_to_disk(dataset).await
    }
}

fn sanitize_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
