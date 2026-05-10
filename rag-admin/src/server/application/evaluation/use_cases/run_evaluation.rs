use std::sync::Arc;

use tokio::sync::RwLock;

use crate::server::application::blog::ports::BlogSource;
use crate::server::application::chunking::PostChunkingService;
use crate::server::application::embedding::EmbeddingService;
use crate::server::application::evaluation::ports::{
    EvaluationDatasetStore, EvaluationResultStore,
};
use crate::server::application::evaluation::progress::EvaluationProgress;
use crate::server::application::evaluation::retrieval::{
    body_eval_chunk, domain_eval_chunk, EvalChunk,
};
use crate::server::application::evaluation::scoring::evaluate_variant;
use crate::server::application::ports::Tokenizer;
use crate::server::application::AppError;
use crate::server::domain::Post;
use crate::shared::{
    plain_f32_vec, ChunkingVariant, EmbeddingModel, EvaluationDatasetDto, EvaluationRunOptions,
    EvaluationRunResult, EvaluationVariantResult, SettingsDto,
};

pub struct RunEvaluationUseCase {
    blog_source: Arc<dyn BlogSource>,
    embedding_service: Arc<EmbeddingService>,
    settings: Arc<RwLock<SettingsDto>>,
    post_chunking_service: Arc<PostChunkingService>,
    tokenizer: Arc<dyn Tokenizer>,
    evaluation_dataset_store: Arc<dyn EvaluationDatasetStore>,
    evaluation_result_store: Arc<dyn EvaluationResultStore>,
}

pub(crate) struct EvaluationRunContext {
    pub post: Post,
    pub dataset: EvaluationDatasetDto,
    pub model: EmbeddingModel,
    pub question_embeddings: Vec<Vec<f32>>,
}

pub(crate) struct PreparedEvaluationVariant {
    pub variant: ChunkingVariant,
    pub eval_chunks: Vec<EvalChunk>,
    pub chunk_embeddings: Vec<Vec<f32>>,
}

impl RunEvaluationUseCase {
    pub fn new(
        blog_source: Arc<dyn BlogSource>,
        embedding_service: Arc<EmbeddingService>,
        settings: Arc<RwLock<SettingsDto>>,
        post_chunking_service: Arc<PostChunkingService>,
        tokenizer: Arc<dyn Tokenizer>,
        evaluation_dataset_store: Arc<dyn EvaluationDatasetStore>,
        evaluation_result_store: Arc<dyn EvaluationResultStore>,
    ) -> Self {
        Self {
            blog_source,
            embedding_service,
            settings,
            post_chunking_service,
            tokenizer,
            evaluation_dataset_store,
            evaluation_result_store,
        }
    }

    pub async fn execute(
        &self,
        slug: &str,
        variants: Vec<ChunkingVariant>,
        options: EvaluationRunOptions,
        progress: Option<Arc<dyn EvaluationProgress>>,
        store_result: bool,
    ) -> Result<EvaluationRunResult, AppError> {
        if variants.is_empty() {
            return Err(AppError::Validation(
                "at least one chunking variant is required".into(),
            ));
        }

        if let Some(ref progress) = progress {
            progress
                .info(format!(
                    "INIT_PROCESS: starting evaluation for post '{slug}'..."
                ))
                .await;
        }

        let context = self
            .prepare_evaluation_context(slug, progress.as_ref())
            .await?;
        let mut variant_results = Vec::with_capacity(variants.len());
        for variant in variants {
            let prepared = self
                .prepare_evaluation_variant(&context, variant, &options, progress.as_ref())
                .await?;
            variant_results.push(score_prepared_variant(&prepared, &context, &options));
        }

        let result = EvaluationRunResult::new(
            context.post.slug().to_string(),
            context.post.version().as_str().to_string(),
            now_rfc3339(),
            options,
            None,
            variant_results,
        );
        if store_result {
            self.evaluation_result_store.store(&result).await?;

            if let Some(progress) = progress.as_ref() {
                progress
                    .success("evaluation complete and saved.".to_string())
                    .await;
            }
        }

        Ok(result)
    }

    pub(crate) async fn prepare_evaluation_context(
        &self,
        slug: &str,
        progress: Option<&Arc<dyn EvaluationProgress>>,
    ) -> Result<EvaluationRunContext, AppError> {
        let blog_post = self.blog_source.fetch(slug).await?;
        let post = Post::try_new(blog_post)?;
        let dataset = self
            .evaluation_dataset_store
            .load(post.slug(), post.version().as_str())
            .await
            .map_err(|e| match e {
                AppError::Io(_) => AppError::NotFound(format!(
                    "no chunking evaluation dataset for {} @ {}",
                    post.slug(),
                    post.version().short()
                )),
                other => other,
            })?;

        if let Some(progress) = progress {
            progress
                .info(format!(
                    "using dataset with {} questions...",
                    dataset.questions.len()
                ))
                .await;
        }

        if dataset.questions.is_empty() {
            return Err(AppError::Validation(
                "evaluation dataset has no questions".into(),
            ));
        }

        let model = self.settings.read().await.embedding_model.clone();

        let question_embeddings =
            if let Some(cached_embeddings) = cached_question_embeddings(&dataset, &model) {
                if let Some(progress) = progress {
                    progress
                        .info(format!(
                            "using cached embeddings for {} questions via {}...",
                            cached_embeddings.len(),
                            model.id
                        ))
                        .await;
                }
                cached_embeddings
            } else {
                if let Some(progress) = progress {
                    progress
                        .info(format!(
                            "embedding {} questions via {}...",
                            dataset.questions.len(),
                            model.id
                        ))
                        .await;
                }

                let question_texts: Vec<String> = dataset
                    .questions
                    .iter()
                    .map(|question| question.question.clone())
                    .collect();
                self.embedding_service
                    .embed_batch(&model, &question_texts)
                    .await?
            };

        Ok(EvaluationRunContext {
            post,
            dataset,
            model,
            question_embeddings,
        })
    }

    pub(crate) async fn prepare_evaluation_variant(
        &self,
        context: &EvaluationRunContext,
        variant: ChunkingVariant,
        options: &EvaluationRunOptions,
        progress: Option<&Arc<dyn EvaluationProgress>>,
    ) -> Result<PreparedEvaluationVariant, AppError> {
        if let Some(progress) = progress {
            progress
                .info(format!("evaluating variant '{}'...", variant.label))
                .await;
        }

        let chunked_post = self
            .post_chunking_service
            .chunk_post(&context.post, &variant.config, options.include_glossary)
            .await?;
        let mut eval_chunks: Vec<EvalChunk> = Vec::new();
        for chunk in chunked_post.body_chunks {
            let token_count = self.tokenizer.count(&chunk.text)?;
            eval_chunks.push(body_eval_chunk(chunk, token_count));
        }
        for chunk in chunked_post.glossary_chunks {
            let token_count = self.tokenizer.count(&chunk.text)?;
            eval_chunks.push(domain_eval_chunk(chunk, token_count));
        }

        if eval_chunks.is_empty() {
            return Err(AppError::Validation(format!(
                "variant '{}' produced no chunks",
                variant.label
            )));
        }

        if let Some(progress) = progress {
            progress
                .info(format!(
                    "variant '{}' produced {} chunks. embedding...",
                    variant.label,
                    eval_chunks.len()
                ))
                .await;
        }

        let chunk_texts: Vec<String> = eval_chunks.iter().map(|chunk| chunk.text.clone()).collect();
        let chunk_embeddings = self
            .embedding_service
            .embed_batch(&context.model, &chunk_texts)
            .await?;

        if let Some(progress) = progress {
            progress
                .info(format!("scoring variant '{}'...", variant.label))
                .await;
        }

        Ok(PreparedEvaluationVariant {
            variant,
            eval_chunks,
            chunk_embeddings,
        })
    }
}

pub(crate) fn score_prepared_variant(
    prepared: &PreparedEvaluationVariant,
    context: &EvaluationRunContext,
    options: &EvaluationRunOptions,
) -> EvaluationVariantResult {
    evaluate_variant(
        prepared.variant.clone(),
        &context.dataset.questions,
        &prepared.eval_chunks,
        &prepared.chunk_embeddings,
        &context.question_embeddings,
        options,
    )
}

pub(crate) fn score_prepared_variant_for_indices(
    prepared: &PreparedEvaluationVariant,
    context: &EvaluationRunContext,
    options: &EvaluationRunOptions,
    indices: &[usize],
) -> EvaluationVariantResult {
    let questions = indices
        .iter()
        .map(|index| context.dataset.questions[*index].clone())
        .collect::<Vec<_>>();
    let question_embeddings = indices
        .iter()
        .map(|index| context.question_embeddings[*index].clone())
        .collect::<Vec<_>>();

    evaluate_variant(
        prepared.variant.clone(),
        &questions,
        &prepared.eval_chunks,
        &prepared.chunk_embeddings,
        &question_embeddings,
        options,
    )
}

pub(crate) fn now_rfc3339() -> String {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn cached_question_embeddings(
    dataset: &EvaluationDatasetDto,
    model: &EmbeddingModel,
) -> Option<Vec<Vec<f32>>> {
    if dataset.embedding_model_backend != Some(model.backend)
        || dataset.embedding_model_id.as_deref() != Some(model.id.as_str())
        || dataset.embedding_model_dims != Some(model.dims)
    {
        return None;
    }

    let expected_dims = model.dims as usize;
    let mut embeddings = Vec::with_capacity(dataset.questions.len());
    for question in &dataset.questions {
        let embedding = question.embedding.as_ref()?;
        if embedding.len() != expected_dims {
            return None;
        }
        embeddings.push(plain_f32_vec(embedding));
    }

    Some(embeddings)
}
