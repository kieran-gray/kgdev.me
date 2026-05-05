use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::{Mutex, RwLock};

use crate::server::application::blog::ports::BlogSource;
use crate::server::application::chunking::PostChunkingService;
use crate::server::application::embedding::EmbeddingService;
use crate::server::application::evaluation::generator::{
    build_question_prompt, generated_to_question, sample_window,
};
use crate::server::application::evaluation::ports::{
    EvaluationDatasetStore, EvaluationGenerator, EvaluationResultStore,
};
use crate::server::application::evaluation::question_filter::filter_generated_questions;
use crate::server::application::evaluation::retrieval::EvalChunk;
use crate::server::application::evaluation::scoring::evaluate_variant;
use crate::server::application::{AppError, IngestLogEvent, Job, JobRegistry};
use crate::server::domain::Post;
use crate::shared::{
    ChunkingVariant, EvaluationDataset, EvaluationDatasetStatus, EvaluationJobInfo,
    EvaluationRunOptions, EvaluationRunResult, SettingsDto,
};

pub struct ChunkingEvaluationService {
    blog_source: Arc<dyn BlogSource>,
    generator: Arc<dyn EvaluationGenerator>,
    embedding_service: Arc<EmbeddingService>,
    settings: Arc<RwLock<SettingsDto>>,
    job_registry: Arc<JobRegistry>,
    post_chunking_service: Arc<PostChunkingService>,
    evaluation_dataset_store: Arc<dyn EvaluationDatasetStore>,
    evaluation_result_store: Arc<dyn EvaluationResultStore>,
    running: Mutex<HashSet<String>>,
}

impl ChunkingEvaluationService {
    pub fn new(
        blog_source: Arc<dyn BlogSource>,
        generator: Arc<dyn EvaluationGenerator>,
        embedding_service: Arc<EmbeddingService>,
        settings: Arc<RwLock<SettingsDto>>,
        job_registry: Arc<JobRegistry>,
        evaluation_dataset_store: Arc<dyn EvaluationDatasetStore>,
        evaluation_result_store: Arc<dyn EvaluationResultStore>,
        post_chunking_service: Arc<PostChunkingService>,
    ) -> Arc<Self> {
        Arc::new(Self {
            blog_source,
            generator,
            embedding_service,
            settings,
            job_registry,
            evaluation_dataset_store,
            evaluation_result_store,
            post_chunking_service,
            running: Mutex::new(HashSet::new()),
        })
    }

    pub async fn start_generate_dataset(
        self: &Arc<Self>,
        slug: String,
    ) -> Result<EvaluationJobInfo, AppError> {
        {
            let mut guard = self.running.lock().await;
            if guard.contains(&slug) {
                return Err(AppError::Validation(format!(
                    "chunking evaluation dataset generation for {slug} is already running"
                )));
            }
            guard.insert(slug.clone());
        }

        let (job_id, job) = self.job_registry.create().await;
        let stream_url = format!("/api/ingest/logs/{job_id}");

        let svc = self.clone();
        let slug_for_task = slug.clone();
        let job_for_task = job.clone();
        tokio::spawn(async move {
            let result = svc
                .run_generate_dataset(&slug_for_task, job_for_task.clone())
                .await;
            if let Err(e) = result {
                job_for_task
                    .emit(IngestLogEvent::error(format!(
                        "chunking evaluation generation failed: {e}"
                    )))
                    .await;
            }
            job_for_task.finish().await;
            svc.running.lock().await.remove(&slug_for_task);
        });

        Ok(EvaluationJobInfo { job_id, stream_url })
    }

    pub async fn dataset_status(&self, slug: &str) -> Result<EvaluationDatasetStatus, AppError> {
        let post = Post::try_new(self.blog_source.fetch(slug).await?)?;
        self.evaluation_dataset_store
            .status(slug, post.version().as_str())
            .await
    }

    pub async fn latest_result(&self, slug: &str) -> Result<Option<EvaluationRunResult>, AppError> {
        let post = Post::try_new(self.blog_source.fetch(slug).await?)?;
        self.evaluation_result_store
            .load(post.slug(), post.version().as_str())
            .await
    }

    pub async fn start_run_evaluation(
        self: &Arc<Self>,
        slug: String,
        variants: Vec<ChunkingVariant>,
        options: EvaluationRunOptions,
    ) -> Result<EvaluationJobInfo, AppError> {
        let (job_id, job) = self.job_registry.create().await;
        let stream_url = format!("/api/ingest/logs/{job_id}");

        let svc = self.clone();
        tokio::spawn(async move {
            let result = svc
                .run_evaluation(&slug, variants, options, Some(job.clone()))
                .await;
            if let Err(e) = result {
                job.emit(IngestLogEvent::error(format!("evaluation failed: {e}")))
                    .await;
            }
            job.finish().await;
        });

        Ok(EvaluationJobInfo { job_id, stream_url })
    }

    pub async fn run_evaluation(
        &self,
        slug: &str,
        variants: Vec<ChunkingVariant>,
        options: EvaluationRunOptions,
        job: Option<Arc<Job>>,
    ) -> Result<EvaluationRunResult, AppError> {
        if variants.is_empty() {
            return Err(AppError::Validation(
                "at least one chunking variant is required".into(),
            ));
        }

        if let Some(ref j) = job {
            j.emit(IngestLogEvent::info(format!(
                "INIT_PROCESS: starting evaluation for post '{slug}'..."
            )))
            .await;
        }

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

        if let Some(ref j) = job {
            j.emit(IngestLogEvent::info(format!(
                "using dataset with {} questions...",
                dataset.questions.len()
            )))
            .await;
        }

        if dataset.questions.is_empty() {
            return Err(AppError::Validation(
                "evaluation dataset has no questions".into(),
            ));
        }

        let model = self.settings.read().await.embedding_model.clone();

        if let Some(ref j) = job {
            j.emit(IngestLogEvent::info(format!(
                "embedding {} questions via {}...",
                dataset.questions.len(),
                model.id
            )))
            .await;
        }

        let question_texts: Vec<String> = dataset
            .questions
            .iter()
            .map(|q| q.question.clone())
            .collect();
        let question_embeddings = self
            .embedding_service
            .embed_batch(&model, &question_texts)
            .await?;

        let mut variant_results = Vec::with_capacity(variants.len());
        for variant in variants {
            if let Some(ref j) = job {
                j.emit(IngestLogEvent::info(format!(
                    "evaluating variant '{}'...",
                    variant.label
                )))
                .await;
            }

            let chunked_post = self
                .post_chunking_service
                .chunk_post(&post, variant.config, options.include_glossary)
                .await?;
            let eval_chunks: Vec<EvalChunk> = chunked_post
                .body_chunks
                .into_iter()
                .map(EvalChunk::from)
                .chain(
                    chunked_post
                        .glossary_chunks
                        .into_iter()
                        .map(EvalChunk::from),
                )
                .collect();

            if eval_chunks.is_empty() {
                return Err(AppError::Validation(format!(
                    "variant '{}' produced no chunks",
                    variant.label
                )));
            }

            if let Some(ref j) = job {
                j.emit(IngestLogEvent::info(format!(
                    "variant '{}' produced {} chunks. embedding...",
                    variant.label,
                    eval_chunks.len()
                )))
                .await;
            }

            let chunk_texts: Vec<String> = eval_chunks.iter().map(|c| c.text.clone()).collect();
            let chunk_embeddings = self
                .embedding_service
                .embed_batch(&model, &chunk_texts)
                .await?;

            if let Some(ref j) = job {
                j.emit(IngestLogEvent::info(format!(
                    "scoring variant '{}'...",
                    variant.label
                )))
                .await;
            }

            let result = evaluate_variant(
                variant,
                &dataset.questions,
                &eval_chunks,
                &chunk_embeddings,
                &question_embeddings,
                &options,
            );
            variant_results.push(result);
        }

        let result = EvaluationRunResult {
            slug: post.slug().to_string(),
            post_version: post.version().as_str().to_string(),
            options,
            variants: variant_results,
        };
        self.evaluation_result_store.store(&result).await?;

        if let Some(ref j) = job {
            j.emit(IngestLogEvent::success("evaluation complete and saved."))
                .await;
        }

        Ok(result)
    }

    async fn run_generate_dataset(&self, slug: &str, job: Arc<Job>) -> Result<(), AppError> {
        job.emit(IngestLogEvent::info(format!(
            "fetching post {slug} for chunking evaluation..."
        )))
        .await;
        let post = Post::try_new(self.blog_source.fetch(slug).await?)?;
        let settings = self.settings.read().await.clone();
        let evaluation_settings = settings.evaluation;
        let embedding_model = settings.embedding_model;
        let target_questions = evaluation_settings.question_count as usize;
        let max_attempts = (target_questions * 4).max(target_questions + 2);

        job.emit(IngestLogEvent::info(format!(
            "generating {target_questions} synthetic question(s) via {} ({})",
            evaluation_settings.generation_backend.as_str(),
            evaluation_settings.generation_model
        )))
        .await;

        let mut questions = Vec::new();
        let mut previous_questions: Vec<String> = Vec::new();

        for attempt in 0..max_attempts {
            if questions.len() >= target_questions {
                break;
            }

            let source_window = sample_window(post.markdown_body(), attempt, max_attempts);
            let prompt = build_question_prompt(&source_window, &previous_questions);
            let generated = self
                .generator
                .generate_question(&evaluation_settings.generation_model, prompt)
                .await;

            match generated {
                Ok(generated) => match generated_to_question(&generated, post.markdown_body()) {
                    Ok(question) => {
                        previous_questions.push(question.question.clone());
                        questions.push(question);
                        job.emit(IngestLogEvent::info(format!(
                            "accepted candidate question {}/{}",
                            questions.len(),
                            target_questions
                        )))
                        .await;
                    }
                    Err(e) => {
                        job.emit(IngestLogEvent::warn(format!(
                            "discarded generated question: {e}"
                        )))
                        .await;
                    }
                },
                Err(e) => {
                    job.emit(IngestLogEvent::warn(format!(
                        "generation attempt {} failed: {e}",
                        attempt + 1
                    )))
                    .await;
                }
            }
        }

        if questions.is_empty() {
            return Err(AppError::Upstream(
                "generator did not produce any usable evaluation questions".into(),
            ));
        }

        let candidate_count = questions.len();
        let (questions, filter_stats) = filter_generated_questions(
            &self.embedding_service,
            &embedding_model,
            questions,
            evaluation_settings.excerpt_similarity_threshold(),
            evaluation_settings.duplicate_similarity_threshold(),
        )
        .await?;
        job.emit(IngestLogEvent::info(format!(
            "filtered candidate questions: kept {}/{}, low excerpt similarity {}, duplicates {}",
            questions.len(),
            candidate_count,
            filter_stats.low_excerpt_similarity,
            filter_stats.duplicate
        )))
        .await;

        if questions.is_empty() {
            return Err(AppError::Upstream(
                "all generated evaluation questions were removed by similarity filters".into(),
            ));
        }

        let dataset = EvaluationDataset {
            slug: post.slug().to_string(),
            post_version: post.version().as_str().to_string(),
            generated_at: now_rfc3339(),
            questions,
        };
        self.evaluation_dataset_store.store(&dataset).await?;

        job.emit(IngestLogEvent::success(format!(
            "saved chunking evaluation dataset: {} question(s)",
            dataset.questions.len()
        )))
        .await;
        Ok(())
    }
}

fn now_rfc3339() -> String {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}
