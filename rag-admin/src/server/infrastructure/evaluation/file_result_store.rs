use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use crate::server::application::evaluation::ports::EvaluationResultStore;
use crate::server::application::AppError;
use crate::shared::{
    evaluation_score, EvaluationResultSplit, EvaluationRunOptions, EvaluationRunResult,
    EvaluationRunSummary, EvaluationVariantResult,
};

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

    fn history_dir(&self, slug: &str, version: &str) -> PathBuf {
        self.path
            .join(sanitize_path_segment(slug))
            .join("results")
            .join(sanitize_path_segment(version))
    }

    fn history_path(&self, slug: &str, version: &str, run_id: &str) -> PathBuf {
        self.history_dir(slug, version)
            .join(format!("{}.json", sanitize_path_segment(run_id)))
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

    async fn list(&self, slug: &str, version: &str) -> Result<Vec<EvaluationRunSummary>, AppError> {
        let mut out = Vec::new();
        let dir = self.history_dir(slug, version);
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if let Some(latest) = self.load(slug, version).await? {
                    out.push(summary_from_result(&latest));
                }
                return Ok(out);
            }
            Err(e) => return Err(AppError::Io(format!("read evaluation result dir: {e}"))),
        };

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| AppError::Io(format!("read evaluation result entry: {e}")))?
        {
            if entry.path().extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let bytes = tokio::fs::read(entry.path())
                .await
                .map_err(|e| AppError::Io(format!("read evaluation result: {e}")))?;
            let result: EvaluationRunResult = serde_json::from_slice(&bytes)
                .map_err(|e| AppError::Internal(format!("parse evaluation result: {e}")))?;
            out.push(summary_from_result(&result));
        }

        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(out)
    }

    async fn load_run(
        &self,
        slug: &str,
        version: &str,
        run_id: &str,
    ) -> Result<Option<EvaluationRunResult>, AppError> {
        if run_id == "latest" {
            return self.load(slug, version).await;
        }
        let path = self.history_path(slug, version, run_id);
        match tokio::fs::read(&path).await {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| AppError::Internal(format!("parse evaluation result: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AppError::Io(format!("read evaluation result: {e}"))),
        }
    }

    async fn store(&self, result: &EvaluationRunResult) -> Result<(), AppError> {
        let latest_path = self.result_path(&result.slug, &result.post_version);
        let history_path = self.history_path(&result.slug, &result.post_version, &result.run_id);
        if let Some(parent) = latest_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AppError::Io(format!("create evaluation result dir: {e}")))?;
        }
        if let Some(parent) = history_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AppError::Io(format!("create evaluation result history dir: {e}")))?;
        }
        let bytes = serde_json::to_vec_pretty(result)
            .map_err(|e| AppError::Internal(format!("encode evaluation result: {e}")))?;
        tokio::fs::write(history_path, &bytes)
            .await
            .map_err(|e| AppError::Io(format!("write evaluation result history: {e}")))?;
        tokio::fs::write(latest_path, bytes)
            .await
            .map_err(|e| AppError::Io(format!("write latest evaluation result: {e}")))
    }
}

fn summary_from_result(result: &EvaluationRunResult) -> EvaluationRunSummary {
    let best = best_summary_variant(&result.variants);
    EvaluationRunSummary {
        run_id: if result.run_id.is_empty() {
            "latest".into()
        } else {
            result.run_id.clone()
        },
        created_at: result.created_at.clone(),
        options: result.options.clone(),
        variant_labels: result
            .variants
            .iter()
            .map(|v| v.variant.label.clone())
            .collect(),
        variant_count: result.variants.len() as u32,
        option_count: option_count(result),
        best_label: best
            .map(|v| v.variant.label.clone())
            .unwrap_or_else(|| "N/A".into()),
        best_score: best.map(|v| evaluation_score(&v.metrics)).unwrap_or(0.0),
        best_recall: best.map(|v| v.metrics.recall_mean).unwrap_or(0.0),
        best_precision: best.map(|v| v.metrics.precision_mean).unwrap_or(0.0),
        best_precision_omega: best.map(|v| v.metrics.precision_omega_mean).unwrap_or(0.0),
    }
}

fn best_summary_variant(variants: &[EvaluationVariantResult]) -> Option<&EvaluationVariantResult> {
    variants
        .iter()
        .find(|variant| variant.selected && variant.split == EvaluationResultSplit::Holdout)
        .or_else(|| {
            variants.iter().max_by(|a, b| {
                evaluation_score(&a.metrics)
                    .partial_cmp(&evaluation_score(&b.metrics))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        })
}

fn option_count(result: &EvaluationRunResult) -> u32 {
    let mut unique = Vec::<EvaluationRunOptions>::new();
    for options in result
        .variants
        .iter()
        .map(|variant| variant.options.clone())
    {
        if !unique.contains(&options) {
            unique.push(options);
        }
    }

    unique.len().max(1) as u32
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
            run_id: "run-1".into(),
            slug: "post/one".into(),
            post_version: "abc123".into(),
            created_at: "2026-05-06T00:00:00Z".into(),
            options: EvaluationRunOptions::default(),
            autotune: None,
            variants: vec![EvaluationVariantResult {
                variant: ChunkingVariant {
                    label: "current".into(),
                    config: ChunkingConfig {
                        strategy: ChunkStrategy::Section,
                        ..ChunkingConfig::default()
                    },
                },
                options: EvaluationRunOptions::default(),
                split: EvaluationResultSplit::Full,
                selected: false,
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
                average_chunk_tokens: 100,
                average_retrieved_tokens: 200,
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

    #[tokio::test]
    async fn stores_result_history() {
        let root = std::env::temp_dir().join(format!(
            "rag-admin-evaluation-result-history-{}",
            std::process::id()
        ));
        let store = FileEvaluationResultStore::new(root);
        let mut first = result();
        first.run_id = "run-1".into();
        let mut second = result();
        second.run_id = "run-2".into();
        second.created_at = "2026-05-06T00:01:00Z".into();

        store.store(&first).await.unwrap();
        store.store(&second).await.unwrap();

        let history = store.list(&first.slug, &first.post_version).await.unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].run_id, "run-2");
        let loaded = store
            .load_run(&first.slug, &first.post_version, "run-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.run_id, "run-1");
    }
}
