use std::sync::Arc;

use crate::server::application::embedding::ports::Embedder;
use crate::server::application::AppError;
use crate::shared::{EmbedResult, EmbeddingModel};

pub struct EmbeddingService {
    embedder: Arc<dyn Embedder>,
}

impl EmbeddingService {
    pub fn new(embedder: Arc<dyn Embedder>) -> Arc<Self> {
        Arc::new(Self { embedder })
    }

    pub async fn embed_batch(
        &self,
        model: &EmbeddingModel,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>, AppError> {
        let vecs = self.embedder.embed_batch(&model.id, texts).await?;
        verify_dims(model, &vecs)?;
        Ok(vecs)
    }

    pub async fn embed_texts(
        &self,
        model: &str,
        text_a: &str,
        text_b: &str,
    ) -> Result<EmbedResult, AppError> {
        let texts = vec![text_a.to_string(), text_b.to_string()];
        let vecs = self.embedder.embed_batch(model, &texts).await?;

        if vecs.len() < 2 || vecs[0].is_empty() {
            return Err(AppError::Internal(
                "embedder returned unexpected result".into(),
            ));
        }

        let a = &vecs[0];
        let b = &vecs[1];

        let norm_a = l2_norm(a);
        let norm_b = l2_norm(b);
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let similarity = if norm_a > 0.0 && norm_b > 0.0 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        };

        Ok(EmbedResult {
            dims: a.len(),
            norm_a,
            norm_b,
            similarity,
        })
    }
}

fn verify_dims(model: &EmbeddingModel, vecs: &[Vec<f32>]) -> Result<(), AppError> {
    if let Some(first) = vecs.first() {
        if first.len() as u32 != model.dims {
            return Err(AppError::Validation(format!(
                "embedder returned dims={} but model '{}' declares dims={}",
                first.len(),
                model.id,
                model.dims
            )));
        }
    }
    Ok(())
}

fn l2_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::server::application::test_support::MockEmbedder;
    use crate::shared::EmbedderBackend;

    fn model(dims: u32) -> EmbeddingModel {
        EmbeddingModel {
            backend: EmbedderBackend::Cloudflare,
            id: "@cf/test/model".into(),
            dims,
        }
    }

    #[tokio::test]
    async fn embed_batch_returns_one_vector_per_text() {
        let embedder = Arc::new(MockEmbedder::new(4));
        let svc = EmbeddingService::new(embedder.clone());

        let vecs = svc
            .embed_batch(&model(4), &["a".into(), "b".into(), "c".into()])
            .await
            .unwrap();

        assert_eq!(vecs.len(), 3);
        assert!(vecs.iter().all(|v| v.len() == 4));
        assert_eq!(embedder.calls().len(), 1);
        assert_eq!(embedder.calls()[0].0, "@cf/test/model");
    }

    #[tokio::test]
    async fn embed_batch_rejects_dim_mismatch() {
        let embedder = Arc::new(MockEmbedder::new(4).with_actual_dims(3));
        let svc = EmbeddingService::new(embedder);

        let err = svc.embed_batch(&model(4), &["x".into()]).await.unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[tokio::test]
    async fn embed_batch_propagates_embedder_failure() {
        let embedder =
            Arc::new(MockEmbedder::new(4).with_failure(AppError::Upstream("boom".into())));
        let svc = EmbeddingService::new(embedder);

        let err = svc.embed_batch(&model(4), &["x".into()]).await.unwrap_err();
        assert!(matches!(err, AppError::Upstream(_)));
    }

    #[tokio::test]
    async fn embed_batch_skips_dim_check_when_empty() {
        let embedder = Arc::new(MockEmbedder::new(4).with_actual_dims(99));
        let svc = EmbeddingService::new(embedder);

        let vecs = svc.embed_batch(&model(4), &[]).await.unwrap();
        assert!(vecs.is_empty());
    }

    #[tokio::test]
    async fn embed_texts_computes_unit_similarity_for_identical_vectors() {
        let embedder = Arc::new(MockEmbedder::new(4));
        let svc = EmbeddingService::new(embedder);

        let res = svc.embed_texts("m", "hello", "world").await.unwrap();

        assert_eq!(res.dims, 4);
        assert!(res.norm_a > 0.0);
        assert!(res.norm_b > 0.0);
    }

    #[tokio::test]
    async fn embed_texts_returns_zero_similarity_for_zero_vector() {
        let embedder = Arc::new(MockEmbedder::new(0));
        let svc = EmbeddingService::new(embedder);

        let err = svc.embed_texts("m", "a", "b").await.unwrap_err();
        assert!(matches!(err, AppError::Internal(_)));
    }
}
