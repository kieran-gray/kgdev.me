use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use crate::server::application::evaluation::ports::EvaluationResultStore;
use crate::server::application::AppError;
use crate::shared::EvaluationRunResult;

pub struct FileEvaluationResultStore {
    path: PathBuf,
}

impl FileEvaluationResultStore {
    pub fn new(path: PathBuf) -> Arc<Self> {
        Arc::new(Self { path })
    }

    fn result_path(&self, slug: &str, version: &str) -> PathBuf {
        self.path
            .join(sanitize_path_segment(slug))
            .join("results")
            .join(format!("{version}.json"))
    }
}

#[async_trait]
impl EvaluationResultStore for FileEvaluationResultStore {
    async fn load(
        &self,
        slug: &str,
        version: &str,
    ) -> Result<Option<EvaluationRunResult>, AppError> {
        let path = self.result_path(slug, version);
        match tokio::fs::read(&path).await {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| AppError::Internal(format!("parse evaluation result: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AppError::Io(format!("read evaluation result: {e}"))),
        }
    }

    async fn store(&self, result: &EvaluationRunResult) -> Result<(), AppError> {
        let path = self.result_path(&result.slug, &result.post_version);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AppError::Io(format!("create evaluation result dir: {e}")))?;
        }
        let bytes = serde_json::to_vec_pretty(result)
            .map_err(|e| AppError::Internal(format!("encode evaluation result: {e}")))?;
        tokio::fs::write(path, bytes)
            .await
            .map_err(|e| AppError::Io(format!("write evaluation result: {e}")))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::{
        ChunkStrategy, ChunkingConfig, ChunkingVariant, EvaluationMetrics, EvaluationRunOptions,
        EvaluationVariantResult,
    };

    fn result() -> EvaluationRunResult {
        EvaluationRunResult {
            slug: "post/one".into(),
            post_version: "abc123".into(),
            options: EvaluationRunOptions::default(),
            variants: vec![EvaluationVariantResult {
                variant: ChunkingVariant {
                    label: "current".into(),
                    config: ChunkingConfig {
                        strategy: ChunkStrategy::Section,
                        ..ChunkingConfig::default()
                    },
                },
                metrics: EvaluationMetrics {
                    recall_mean: 1.0,
                    recall_std: 0.0,
                    precision_mean: 0.8,
                    precision_std: 0.0,
                    iou_mean: 0.7,
                    iou_std: 0.0,
                    precision_omega_mean: 0.9,
                    precision_omega_std: 0.0,
                },
                chunk_count: 3,
                average_chunk_chars: 100,
                average_retrieved_chars: 200,
                question_results: Vec::new(),
            }],
        }
    }

    #[tokio::test]
    async fn stores_and_loads_result_for_slug_and_version() {
        let root = std::env::temp_dir().join(format!(
            "rag-admin-evaluation-result-store-{}",
            std::process::id()
        ));
        let store = FileEvaluationResultStore::new(root);
        let result = result();

        store.store(&result).await.unwrap();
        let loaded = store
            .load(&result.slug, &result.post_version)
            .await
            .unwrap();

        assert_eq!(loaded.unwrap().post_version, result.post_version);
    }
}
