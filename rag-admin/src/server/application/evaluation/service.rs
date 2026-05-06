use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::{Mutex, RwLock};

use crate::server::application::blog::ports::BlogSource;
use crate::server::application::chunking::PostChunkingService;
use crate::server::application::embedding::EmbeddingService;
use crate::server::application::evaluation::generator::build_question_prompt;
use crate::server::application::evaluation::ports::{
    EvaluationDatasetStore, EvaluationGenerator, EvaluationResultStore,
};
use crate::server::application::evaluation::reference_locator::ReferenceLocator;
use crate::server::application::evaluation::retrieval::cosine_similarity;
use crate::server::application::evaluation::retrieval::{
    body_eval_chunk, domain_eval_chunk, EvalChunk,
};
use crate::server::application::evaluation::scoring::evaluate_variant;
use crate::server::application::ports::Tokenizer;
use crate::server::application::{AppError, IngestLogEvent, Job, JobRegistry};
use crate::server::domain::Post;
use crate::shared::{
    evaluation_score, ordered_f32_vec, plain_f32_vec, ChunkingConfig, ChunkingVariant,
    EmbeddingModel, EvaluationAutotuneRequest, EvaluationAutotuneSummary, EvaluationDataset,
    EvaluationDatasetStatus, EvaluationJobInfo, EvaluationQuestion, EvaluationResultSplit,
    EvaluationRunOptions, EvaluationRunResult, EvaluationRunSummary, EvaluationVariantResult,
    SettingsDto,
};

const DATASET_GENERATION_ATTEMPT_MULTIPLIER: usize = 12;
const PREVIOUS_QUESTION_PROMPT_LIMIT: usize = 12;

pub struct ChunkingEvaluationService {
    blog_source: Arc<dyn BlogSource>,
    generator: Arc<dyn EvaluationGenerator>,
    embedding_service: Arc<EmbeddingService>,
    settings: Arc<RwLock<SettingsDto>>,
    job_registry: Arc<JobRegistry>,
    post_chunking_service: Arc<PostChunkingService>,
    tokenizer: Arc<dyn Tokenizer>,
    evaluation_dataset_store: Arc<dyn EvaluationDatasetStore>,
    evaluation_result_store: Arc<dyn EvaluationResultStore>,
    running: Mutex<HashSet<String>>,
}

pub struct ChunkingEvaluationServiceDeps {
    pub blog_source: Arc<dyn BlogSource>,
    pub generator: Arc<dyn EvaluationGenerator>,
    pub embedding_service: Arc<EmbeddingService>,
    pub settings: Arc<RwLock<SettingsDto>>,
    pub job_registry: Arc<JobRegistry>,
    pub evaluation_dataset_store: Arc<dyn EvaluationDatasetStore>,
    pub evaluation_result_store: Arc<dyn EvaluationResultStore>,
    pub post_chunking_service: Arc<PostChunkingService>,
    pub tokenizer: Arc<dyn Tokenizer>,
}

struct EvaluationRunContext {
    post: Post,
    dataset: EvaluationDataset,
    model: EmbeddingModel,
    question_embeddings: Vec<Vec<f32>>,
}

struct PreparedEvaluationVariant {
    variant: ChunkingVariant,
    eval_chunks: Vec<EvalChunk>,
    chunk_embeddings: Vec<Vec<f32>>,
}

#[derive(Clone)]
struct AutotuneCandidate {
    variant: ChunkingVariant,
    options: EvaluationRunOptions,
    tuning_score: f32,
}

impl ChunkingEvaluationService {
    pub fn new(deps: ChunkingEvaluationServiceDeps) -> Arc<Self> {
        Arc::new(Self {
            blog_source: deps.blog_source,
            generator: deps.generator,
            embedding_service: deps.embedding_service,
            settings: deps.settings,
            job_registry: deps.job_registry,
            evaluation_dataset_store: deps.evaluation_dataset_store,
            evaluation_result_store: deps.evaluation_result_store,
            post_chunking_service: deps.post_chunking_service,
            tokenizer: deps.tokenizer,
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

    pub async fn result_history(&self, slug: &str) -> Result<Vec<EvaluationRunSummary>, AppError> {
        let post = Post::try_new(self.blog_source.fetch(slug).await?)?;
        self.evaluation_result_store
            .list(post.slug(), post.version().as_str())
            .await
    }

    pub async fn result_run(
        &self,
        slug: &str,
        run_id: &str,
    ) -> Result<Option<EvaluationRunResult>, AppError> {
        let post = Post::try_new(self.blog_source.fetch(slug).await?)?;
        self.evaluation_result_store
            .load_run(post.slug(), post.version().as_str(), run_id)
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
                .run_evaluation_inner(&slug, variants, options, Some(job.clone()), true)
                .await;
            if let Err(e) = result {
                job.emit(IngestLogEvent::error(format!("evaluation failed: {e}")))
                    .await;
            }
            job.finish().await;
        });

        Ok(EvaluationJobInfo { job_id, stream_url })
    }

    pub async fn start_run_evaluation_matrix(
        self: &Arc<Self>,
        slug: String,
        variant: ChunkingVariant,
        option_sets: Vec<EvaluationRunOptions>,
    ) -> Result<EvaluationJobInfo, AppError> {
        if option_sets.is_empty() {
            return Err(AppError::Validation(
                "at least one evaluation option set is required".into(),
            ));
        }

        let (job_id, job) = self.job_registry.create().await;
        let stream_url = format!("/api/ingest/logs/{job_id}");

        let svc = self.clone();
        tokio::spawn(async move {
            if let Err(e) = svc
                .run_matrix_evaluation_inner(&slug, variant, option_sets, job.clone())
                .await
            {
                job.emit(IngestLogEvent::error(format!("evaluation failed: {e}")))
                    .await;
            }
            job.finish().await;
        });

        Ok(EvaluationJobInfo { job_id, stream_url })
    }

    pub async fn start_run_evaluation_autotune(
        self: &Arc<Self>,
        slug: String,
        request: EvaluationAutotuneRequest,
    ) -> Result<EvaluationJobInfo, AppError> {
        validate_autotune_request(&request)?;

        let (job_id, job) = self.job_registry.create().await;
        let stream_url = format!("/api/ingest/logs/{job_id}");

        let svc = self.clone();
        tokio::spawn(async move {
            if let Err(e) = svc
                .run_autotune_evaluation_inner(&slug, request, job.clone())
                .await
            {
                job.emit(IngestLogEvent::error(format!("autotune failed: {e}")))
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
        self.run_evaluation_inner(slug, variants, options, job, true)
            .await
    }

    async fn run_evaluation_inner(
        &self,
        slug: &str,
        variants: Vec<ChunkingVariant>,
        options: EvaluationRunOptions,
        job: Option<Arc<Job>>,
        store_result: bool,
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

        let context = self.prepare_evaluation_context(slug, job.as_ref()).await?;
        let mut variant_results = Vec::with_capacity(variants.len());
        for variant in variants {
            let prepared = self
                .prepare_evaluation_variant(&context, variant, &options, job.as_ref())
                .await?;
            variant_results.push(score_prepared_variant(&prepared, &context, &options));
        }

        let result = EvaluationRunResult {
            run_id: new_run_id(),
            slug: context.post.slug().to_string(),
            post_version: context.post.version().as_str().to_string(),
            created_at: now_rfc3339(),
            options,
            autotune: None,
            variants: variant_results,
        };
        if store_result {
            self.evaluation_result_store.store(&result).await?;

            if let Some(j) = job.as_ref() {
                j.emit(IngestLogEvent::success("evaluation complete and saved."))
                    .await;
            }
        }

        Ok(result)
    }

    async fn run_matrix_evaluation_inner(
        &self,
        slug: &str,
        variant: ChunkingVariant,
        option_sets: Vec<EvaluationRunOptions>,
        job: Arc<Job>,
    ) -> Result<(), AppError> {
        let total = option_sets.len();
        let first_options = option_sets.first().cloned().ok_or_else(|| {
            AppError::Validation("at least one evaluation option set is required".into())
        })?;

        job.emit(IngestLogEvent::info(format!(
            "INIT_PROCESS: starting matrix evaluation for post '{slug}'..."
        )))
        .await;

        let context = self.prepare_evaluation_context(slug, Some(&job)).await?;
        let mut prepared_with_glossary: Option<PreparedEvaluationVariant> = None;
        let mut prepared_without_glossary: Option<PreparedEvaluationVariant> = None;
        let mut matrix_variants = Vec::with_capacity(total);

        for (index, options) in option_sets.iter().enumerate() {
            job.emit(IngestLogEvent::info(format!(
                "PARAM_SWEEP: scoring {}/{} for '{}' with TOP_K={} MIN_SCORE={}",
                index + 1,
                total,
                variant.label,
                options.top_k,
                options.min_score_milli
            )))
            .await;

            let prepared = if options.include_glossary {
                if prepared_with_glossary.is_none() {
                    prepared_with_glossary = Some(
                        self.prepare_evaluation_variant(
                            &context,
                            variant.clone(),
                            options,
                            Some(&job),
                        )
                        .await?,
                    );
                }
                prepared_with_glossary
                    .as_ref()
                    .expect("prepared variant should exist")
            } else {
                if prepared_without_glossary.is_none() {
                    prepared_without_glossary = Some(
                        self.prepare_evaluation_variant(
                            &context,
                            variant.clone(),
                            options,
                            Some(&job),
                        )
                        .await?,
                    );
                }
                prepared_without_glossary
                    .as_ref()
                    .expect("prepared variant should exist")
            };

            let mut variant_result = score_prepared_variant(prepared, &context, options);
            variant_result.variant.label =
                matrix_variant_label(&variant_result.variant.label, options);
            matrix_variants.push(variant_result);
        }

        let result = EvaluationRunResult {
            run_id: new_run_id(),
            slug: context.post.slug().to_string(),
            post_version: context.post.version().as_str().to_string(),
            created_at: now_rfc3339(),
            options: first_options,
            autotune: None,
            variants: matrix_variants,
        };
        self.evaluation_result_store.store(&result).await?;

        job.emit(IngestLogEvent::success(format!(
            "matrix evaluation complete and saved as one run with {} result(s).",
            result.variants.len()
        )))
        .await;
        Ok(())
    }

    async fn run_autotune_evaluation_inner(
        &self,
        slug: &str,
        request: EvaluationAutotuneRequest,
        job: Arc<Job>,
    ) -> Result<(), AppError> {
        let variants = autotune_variants(request.current_config);
        let option_sets = autotune_option_sets(&request);
        let candidate_count = variants.len() * option_sets.len();

        job.emit(IngestLogEvent::info(format!(
            "INIT_PROCESS: autotuning post '{slug}' across {} chunker(s), {} option set(s), {} candidate(s)...",
            variants.len(),
            option_sets.len(),
            candidate_count
        )))
        .await;

        let context = self.prepare_evaluation_context(slug, Some(&job)).await?;
        let (tuning_indices, holdout_indices) = tuning_holdout_indices(
            context.dataset.questions.len(),
            context.post.version().as_str(),
        );

        if tuning_indices.is_empty() || holdout_indices.is_empty() {
            return Err(AppError::Validation(
                "autotune requires at least two evaluation questions".into(),
            ));
        }

        job.emit(IngestLogEvent::info(format!(
            "AUTOTUNE_SPLIT: tuning={} holdout={} selection uses tuning only",
            tuning_indices.len(),
            holdout_indices.len()
        )))
        .await;

        let mut tuning_results = Vec::with_capacity(candidate_count);
        let mut best_candidate: Option<AutotuneCandidate> = None;
        let mut evaluated = 0usize;

        for variant in variants {
            let mut prepared_with_glossary: Option<PreparedEvaluationVariant> = None;
            let mut prepared_without_glossary: Option<PreparedEvaluationVariant> = None;

            for options in &option_sets {
                evaluated += 1;
                job.emit(IngestLogEvent::info(format!(
                    "AUTOTUNE_CANDIDATE: {}/{} {} TOP_K={} MIN_SCORE={} GLOSSARY={}",
                    evaluated,
                    candidate_count,
                    variant.label,
                    options.top_k,
                    options.min_score_milli,
                    options.include_glossary
                )))
                .await;

                let prepared = if options.include_glossary {
                    if prepared_with_glossary.is_none() {
                        prepared_with_glossary = Some(
                            self.prepare_evaluation_variant(
                                &context,
                                variant.clone(),
                                options,
                                Some(&job),
                            )
                            .await?,
                        );
                    }
                    prepared_with_glossary
                        .as_ref()
                        .expect("prepared variant should exist")
                } else {
                    if prepared_without_glossary.is_none() {
                        prepared_without_glossary = Some(
                            self.prepare_evaluation_variant(
                                &context,
                                variant.clone(),
                                options,
                                Some(&job),
                            )
                            .await?,
                        );
                    }
                    prepared_without_glossary
                        .as_ref()
                        .expect("prepared variant should exist")
                };

                let mut result = score_prepared_variant_for_indices(
                    prepared,
                    &context,
                    options,
                    &tuning_indices,
                );
                result.split = EvaluationResultSplit::Tuning;
                let score = evaluation_score(&result.metrics);
                if best_candidate
                    .as_ref()
                    .map(|best| score > best.tuning_score)
                    .unwrap_or(true)
                {
                    best_candidate = Some(AutotuneCandidate {
                        variant: result.variant.clone(),
                        options: options.clone(),
                        tuning_score: score,
                    });
                }
                tuning_results.push(result);
            }
        }

        let Some(best_candidate) = best_candidate else {
            return Err(AppError::Validation(
                "autotune produced no candidate results".into(),
            ));
        };

        job.emit(IngestLogEvent::info(format!(
            "AUTOTUNE_SELECTED: {} TOP_K={} MIN_SCORE={} GLOSSARY={} TUNING_SCORE={:.1}%",
            best_candidate.variant.label,
            best_candidate.options.top_k,
            best_candidate.options.min_score_milli,
            best_candidate.options.include_glossary,
            best_candidate.tuning_score * 100.0
        )))
        .await;

        let prepared = self
            .prepare_evaluation_variant(
                &context,
                best_candidate.variant.clone(),
                &best_candidate.options,
                Some(&job),
            )
            .await?;
        let mut holdout_result = score_prepared_variant_for_indices(
            &prepared,
            &context,
            &best_candidate.options,
            &holdout_indices,
        );
        holdout_result.split = EvaluationResultSplit::Holdout;
        holdout_result.selected = true;
        let holdout_score = evaluation_score(&holdout_result.metrics);

        job.emit(IngestLogEvent::info(format!(
            "AUTOTUNE_HOLDOUT: SCORE={:.1}% RECALL={:.1}% PRECISION={:.1}%",
            holdout_score * 100.0,
            holdout_result.metrics.recall_mean * 100.0,
            holdout_result.metrics.precision_mean * 100.0
        )))
        .await;

        let mut variants = Vec::with_capacity(tuning_results.len() + 1);
        variants.push(holdout_result);
        variants.extend(tuning_results);

        let result = EvaluationRunResult {
            run_id: new_run_id(),
            slug: context.post.slug().to_string(),
            post_version: context.post.version().as_str().to_string(),
            created_at: now_rfc3339(),
            options: best_candidate.options.clone(),
            autotune: Some(EvaluationAutotuneSummary {
                tuning_question_count: tuning_indices.len() as u32,
                holdout_question_count: holdout_indices.len() as u32,
                candidate_count: candidate_count as u32,
                selected_label: best_candidate.variant.label.clone(),
                selected_options: best_candidate.options.clone(),
                selected_config: best_candidate.variant.config,
                tuning_score: best_candidate.tuning_score,
                holdout_score,
            }),
            variants,
        };
        self.evaluation_result_store.store(&result).await?;

        job.emit(IngestLogEvent::success(format!(
            "autotune complete and saved. selected {} with holdout score {:.1}%",
            best_candidate.variant.label,
            holdout_score * 100.0
        )))
        .await;
        Ok(())
    }

    async fn prepare_evaluation_context(
        &self,
        slug: &str,
        job: Option<&Arc<Job>>,
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

        if let Some(j) = job {
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

        let question_embeddings =
            if let Some(cached_embeddings) = cached_question_embeddings(&dataset, &model) {
                if let Some(j) = job {
                    j.emit(IngestLogEvent::info(format!(
                        "using cached embeddings for {} questions via {}...",
                        cached_embeddings.len(),
                        model.id
                    )))
                    .await;
                }
                cached_embeddings
            } else {
                if let Some(j) = job {
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

    async fn prepare_evaluation_variant(
        &self,
        context: &EvaluationRunContext,
        variant: ChunkingVariant,
        options: &EvaluationRunOptions,
        job: Option<&Arc<Job>>,
    ) -> Result<PreparedEvaluationVariant, AppError> {
        if let Some(j) = job {
            j.emit(IngestLogEvent::info(format!(
                "evaluating variant '{}'...",
                variant.label
            )))
            .await;
        }

        let chunked_post = self
            .post_chunking_service
            .chunk_post(&context.post, variant.config, options.include_glossary)
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

        if let Some(j) = job {
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
            .embed_batch(&context.model, &chunk_texts)
            .await?;

        if let Some(j) = job {
            j.emit(IngestLogEvent::info(format!(
                "scoring variant '{}'...",
                variant.label
            )))
            .await;
        }

        Ok(PreparedEvaluationVariant {
            variant,
            eval_chunks,
            chunk_embeddings,
        })
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
        let max_attempts =
            (target_questions * DATASET_GENERATION_ATTEMPT_MULTIPLIER).max(target_questions + 12);

        job.emit(IngestLogEvent::info(format!(
            "generating {target_questions} synthetic question(s) via {} ({}) with up to {max_attempts} attempts",
            evaluation_settings.generation_backend.as_str(),
            evaluation_settings.generation_model
        )))
        .await;

        let mut gate = DatasetQuestionGate::new(
            self.embedding_service.as_ref(),
            &embedding_model,
            evaluation_settings.excerpt_similarity_threshold(),
            evaluation_settings.duplicate_similarity_threshold(),
        );
        let mut previous_coverage: Vec<String> = Vec::new();

        for attempt in 0..max_attempts {
            if gate.kept_count() >= target_questions {
                break;
            }

            let prompt = build_question_prompt(
                post.markdown_body(),
                recent_previous_coverage(&previous_coverage),
            );
            let generated = self
                .generator
                .generate_question(&evaluation_settings.generation_model, prompt)
                .await;

            match generated {
                Ok(generated) => {
                    match ReferenceLocator::generated_to_question(&generated, post.markdown_body())
                    {
                        Ok(question) => {
                            let decision = gate.try_accept(question).await?;
                            match decision {
                                CandidateDecision::Accepted { kept } => {
                                    if let Some(question) = gate.latest_question() {
                                        previous_coverage.push(previous_coverage_entry(question));
                                    }
                                    job.emit(IngestLogEvent::info(format!(
                                        "accepted evaluation question {kept}/{target_questions}"
                                    )))
                                    .await;
                                }
                                CandidateDecision::RejectedLowExcerptSimilarity { similarity } => {
                                    job.emit(IngestLogEvent::warn(format!(
                                    "discarded generated question: low excerpt similarity {:.1}%",
                                    similarity * 100.0
                                )))
                                    .await;
                                }
                                CandidateDecision::RejectedDuplicate { similarity } => {
                                    job.emit(IngestLogEvent::warn(format!(
                                        "discarded generated question: duplicate similarity {:.1}%",
                                        similarity * 100.0
                                    )))
                                    .await;
                                }
                            }
                        }
                        Err(e) => {
                            job.emit(IngestLogEvent::warn(format!(
                                "discarded generated question: {e}"
                            )))
                            .await;
                        }
                    }
                }
                Err(e) => {
                    job.emit(IngestLogEvent::warn(format!(
                        "generation attempt {} failed: {e}",
                        attempt + 1
                    )))
                    .await;
                }
            }
        }

        if gate.generated_count == 0 {
            return Err(AppError::Upstream(
                "generator did not produce any usable evaluation questions".into(),
            ));
        }

        let stats = gate.stats();
        job.emit(IngestLogEvent::info(format!(
            "filtered generated questions: kept {}/{}, low excerpt similarity {}, duplicates {}",
            gate.kept_count(),
            gate.generated_count,
            stats.low_excerpt_similarity,
            stats.duplicate
        )))
        .await;

        if gate.kept_count() < target_questions {
            return Err(AppError::Upstream(format!(
                "generator produced only {}/{} usable evaluation questions after {} attempts",
                gate.kept_count(),
                target_questions,
                max_attempts
            )));
        }

        let questions = gate.into_questions(target_questions);

        let dataset = EvaluationDataset {
            slug: post.slug().to_string(),
            post_version: post.version().as_str().to_string(),
            generated_at: now_rfc3339(),
            embedding_model_backend: Some(embedding_model.backend),
            embedding_model_id: Some(embedding_model.id.clone()),
            embedding_model_dims: Some(embedding_model.dims),
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

fn recent_previous_coverage(previous_coverage: &[String]) -> &[String] {
    let start = previous_coverage
        .len()
        .saturating_sub(PREVIOUS_QUESTION_PROMPT_LIMIT);
    &previous_coverage[start..]
}

fn previous_coverage_entry(question: &EvaluationQuestion) -> String {
    let references = question
        .references
        .iter()
        .take(2)
        .map(|reference| truncate_for_prompt(&reference.content, 160))
        .collect::<Vec<_>>()
        .join(" || ");

    format!(
        "Q: {} | Covered: {}",
        truncate_for_prompt(&question.question, 120),
        references
    )
}

fn truncate_for_prompt(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn validate_autotune_request(request: &EvaluationAutotuneRequest) -> Result<(), AppError> {
    if request.top_k_values.is_empty()
        || request.min_score_milli_values.is_empty()
        || request.include_glossary_values.is_empty()
    {
        return Err(AppError::Validation(
            "autotune requires top_k, min_score, and glossary value ranges".into(),
        ));
    }
    if request.top_k_values.contains(&0) {
        return Err(AppError::Validation(
            "autotune top_k values must be at least 1".into(),
        ));
    }
    if request
        .min_score_milli_values
        .iter()
        .any(|value| *value > 1000)
    {
        return Err(AppError::Validation(
            "autotune min_score values must be between 0 and 1000".into(),
        ));
    }
    Ok(())
}

fn autotune_variants(current_config: ChunkingConfig) -> Vec<ChunkingVariant> {
    ChunkingConfig::sweep_configs(current_config)
        .into_iter()
        .map(|config| ChunkingVariant {
            label: config.display_label(),
            config,
        })
        .collect()
}

fn autotune_option_sets(request: &EvaluationAutotuneRequest) -> Vec<EvaluationRunOptions> {
    let mut option_sets = Vec::new();
    for top_k in unique_u32_values(&request.top_k_values) {
        for min_score_milli in unique_u32_values(&request.min_score_milli_values) {
            for include_glossary in unique_bool_values(&request.include_glossary_values) {
                option_sets.push(EvaluationRunOptions {
                    top_k,
                    min_score_milli,
                    include_glossary,
                });
            }
        }
    }
    option_sets
}

fn unique_u32_values(values: &[u32]) -> Vec<u32> {
    let mut unique = Vec::new();
    for value in values {
        if !unique.contains(value) {
            unique.push(*value);
        }
    }
    unique
}

fn unique_bool_values(values: &[bool]) -> Vec<bool> {
    let mut unique = Vec::new();
    for value in values {
        if !unique.contains(value) {
            unique.push(*value);
        }
    }
    unique
}

fn tuning_holdout_indices(question_count: usize, seed: &str) -> (Vec<usize>, Vec<usize>) {
    if question_count < 2 {
        return ((0..question_count).collect(), Vec::new());
    }

    let holdout_count = ((question_count as f32 * 0.25).ceil() as usize)
        .max(1)
        .min(question_count - 1);
    let mut ranked = (0..question_count)
        .map(|index| (stable_split_score(seed, index), index))
        .collect::<Vec<_>>();
    ranked.sort_by_key(|(score, _)| *score);

    let mut holdout = ranked
        .iter()
        .take(holdout_count)
        .map(|(_, index)| *index)
        .collect::<Vec<_>>();
    let mut tuning = ranked
        .iter()
        .skip(holdout_count)
        .map(|(_, index)| *index)
        .collect::<Vec<_>>();
    holdout.sort_unstable();
    tuning.sort_unstable();

    (tuning, holdout)
}

fn stable_split_score(seed: &str, index: usize) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in seed.bytes().chain(index.to_le_bytes()) {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn matrix_variant_label(label: &str, _options: &EvaluationRunOptions) -> String {
    label.to_string()
}

fn score_prepared_variant(
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

fn score_prepared_variant_for_indices(
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

fn cached_question_embeddings(
    dataset: &EvaluationDataset,
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

#[derive(Debug, Clone, Copy, Default)]
struct DatasetQuestionGateStats {
    low_excerpt_similarity: u32,
    duplicate: u32,
}

enum CandidateDecision {
    Accepted { kept: usize },
    RejectedLowExcerptSimilarity { similarity: f32 },
    RejectedDuplicate { similarity: f32 },
}

struct DatasetQuestionGate<'a> {
    embedding_service: &'a EmbeddingService,
    embedding_model: &'a EmbeddingModel,
    excerpt_similarity_threshold: f32,
    duplicate_similarity_threshold: f32,
    questions: Vec<EvaluationQuestion>,
    question_embeddings: Vec<Vec<f32>>,
    stats: DatasetQuestionGateStats,
    generated_count: usize,
}

impl<'a> DatasetQuestionGate<'a> {
    fn new(
        embedding_service: &'a EmbeddingService,
        embedding_model: &'a EmbeddingModel,
        excerpt_similarity_threshold: f32,
        duplicate_similarity_threshold: f32,
    ) -> Self {
        Self {
            embedding_service,
            embedding_model,
            excerpt_similarity_threshold,
            duplicate_similarity_threshold,
            questions: Vec::new(),
            question_embeddings: Vec::new(),
            stats: DatasetQuestionGateStats::default(),
            generated_count: 0,
        }
    }

    async fn try_accept(
        &mut self,
        question: EvaluationQuestion,
    ) -> Result<CandidateDecision, AppError> {
        self.generated_count += 1;

        let mut texts = Vec::with_capacity(question.references.len() + 1);
        texts.push(question.question.clone());
        texts.extend(question.references.iter().map(|r| r.content.clone()));

        let embeddings = self
            .embedding_service
            .embed_batch(self.embedding_model, &texts)
            .await?;
        let Some(question_embedding) = embeddings.first().cloned() else {
            self.stats.low_excerpt_similarity += 1;
            return Ok(CandidateDecision::RejectedLowExcerptSimilarity { similarity: 0.0 });
        };

        let min_reference_similarity = embeddings
            .iter()
            .skip(1)
            .map(|reference_embedding| cosine_similarity(&question_embedding, reference_embedding))
            .fold(f32::INFINITY, f32::min);

        if !min_reference_similarity.is_finite()
            || min_reference_similarity < self.excerpt_similarity_threshold
        {
            self.stats.low_excerpt_similarity += 1;
            return Ok(CandidateDecision::RejectedLowExcerptSimilarity {
                similarity: min_reference_similarity.max(0.0),
            });
        }

        let max_duplicate_similarity = self
            .question_embeddings
            .iter()
            .map(|kept_embedding| cosine_similarity(&question_embedding, kept_embedding))
            .fold(0.0, f32::max);

        if max_duplicate_similarity >= self.duplicate_similarity_threshold {
            self.stats.duplicate += 1;
            return Ok(CandidateDecision::RejectedDuplicate {
                similarity: max_duplicate_similarity,
            });
        }

        let mut question = question;
        question.embedding = Some(ordered_f32_vec(question_embedding.clone()));
        for (reference, reference_embedding) in question
            .references
            .iter_mut()
            .zip(embeddings.iter().skip(1))
        {
            reference.embedding = Some(ordered_f32_vec(reference_embedding.clone()));
        }

        self.question_embeddings.push(question_embedding);
        self.questions.push(question);
        Ok(CandidateDecision::Accepted {
            kept: self.questions.len(),
        })
    }

    fn kept_count(&self) -> usize {
        self.questions.len()
    }

    fn stats(&self) -> DatasetQuestionGateStats {
        self.stats
    }

    fn latest_question(&self) -> Option<&EvaluationQuestion> {
        self.questions.last()
    }

    fn into_questions(mut self, target_questions: usize) -> Vec<EvaluationQuestion> {
        self.questions.truncate(target_questions);
        self.questions
    }
}

fn now_rfc3339() -> String {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn new_run_id() -> String {
    use time::OffsetDateTime;
    format!("run-{}", OffsetDateTime::now_utc().unix_timestamp_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::{ChunkStrategy, EvaluationReference};

    #[test]
    fn tuning_holdout_split_is_deterministic_and_non_overlapping() {
        let (first_tuning, first_holdout) = tuning_holdout_indices(10, "post-version");
        let (second_tuning, second_holdout) = tuning_holdout_indices(10, "post-version");

        assert_eq!(first_tuning, second_tuning);
        assert_eq!(first_holdout, second_holdout);
        assert_eq!(first_tuning.len(), 7);
        assert_eq!(first_holdout.len(), 3);
        assert!(first_tuning
            .iter()
            .all(|index| !first_holdout.contains(index)));
    }

    #[test]
    fn autotune_option_sets_deduplicate_grid_values() {
        let request = EvaluationAutotuneRequest {
            current_config: ChunkingConfig {
                strategy: ChunkStrategy::Section,
                ..ChunkingConfig::default()
            },
            top_k_values: vec![3, 3, 5],
            min_score_milli_values: vec![0, 700, 700],
            include_glossary_values: vec![true, false, true],
        };

        let option_sets = autotune_option_sets(&request);

        assert_eq!(option_sets.len(), 8);
        assert!(option_sets.contains(&EvaluationRunOptions {
            top_k: 3,
            min_score_milli: 700,
            include_glossary: false,
        }));
    }

    #[test]
    fn previous_coverage_entry_includes_question_and_reference_summary() {
        let question = EvaluationQuestion {
            question: "What increments the in-memory total?".into(),
            references: vec![EvaluationReference {
                content: "When the fetch handler receives a WebSocket upgrade request, it increments the in-memory total.".into(),
                char_start: 0,
                char_end: 0,
                embedding: None,
            }],
            embedding: None,
        };

        let entry = previous_coverage_entry(&question);

        assert!(entry.contains("Q: What increments the in-memory total?"));
        assert!(
            entry.contains("Covered: When the fetch handler receives a WebSocket upgrade request")
        );
    }

    #[test]
    fn recent_previous_coverage_returns_only_latest_entries() {
        let values = (0..20).map(|idx| format!("item-{idx}")).collect::<Vec<_>>();

        let recent = recent_previous_coverage(&values);

        assert_eq!(recent.len(), PREVIOUS_QUESTION_PROMPT_LIMIT);
        assert_eq!(recent.first().unwrap(), "item-8");
        assert_eq!(recent.last().unwrap(), "item-19");
    }
}
