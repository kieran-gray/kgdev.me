use std::sync::Arc;

use tokio::sync::RwLock;

use crate::server::application::blog::ports::BlogSource;
use crate::server::application::chunking::PostChunkingService;
use crate::server::application::embedding::EmbeddingService;
use crate::server::application::evaluation::ports::{
    EvaluationDatasetStore, EvaluationGenerator, EvaluationResultStore,
};
use crate::server::application::evaluation::progress::EvaluationProgress;
use crate::server::application::evaluation::use_cases::generate_synthetic_dataset::GenerateSyntheticDatasetUseCase;
use crate::server::application::evaluation::use_cases::run_autotune_evaluation::RunAutotuneEvaluationUseCase;
use crate::server::application::evaluation::use_cases::run_evaluation::RunEvaluationUseCase;
use crate::server::application::evaluation::use_cases::run_matrix_evaluation::RunMatrixEvaluationUseCase;
use crate::server::application::ports::Tokenizer;
use crate::server::application::{AppError, Job};
use crate::server::domain::Post;
use crate::shared::{
    ChunkingVariant, EvaluationAutotuneRequest, EvaluationDatasetStatus, EvaluationRunOptions,
    EvaluationRunResult, EvaluationRunSummary, SettingsDto,
};

pub struct ChunkingEvaluationService {
    blog_source: Arc<dyn BlogSource>,
    generator: Arc<dyn EvaluationGenerator>,
    embedding_service: Arc<EmbeddingService>,
    settings: Arc<RwLock<SettingsDto>>,
    post_chunking_service: Arc<PostChunkingService>,
    tokenizer: Arc<dyn Tokenizer>,
    evaluation_dataset_store: Arc<dyn EvaluationDatasetStore>,
    evaluation_result_store: Arc<dyn EvaluationResultStore>,
}

pub struct ChunkingEvaluationServiceDeps {
    pub blog_source: Arc<dyn BlogSource>,
    pub generator: Arc<dyn EvaluationGenerator>,
    pub embedding_service: Arc<EmbeddingService>,
    pub settings: Arc<RwLock<SettingsDto>>,
    pub evaluation_dataset_store: Arc<dyn EvaluationDatasetStore>,
    pub evaluation_result_store: Arc<dyn EvaluationResultStore>,
    pub post_chunking_service: Arc<PostChunkingService>,
    pub tokenizer: Arc<dyn Tokenizer>,
}

impl ChunkingEvaluationService {
    pub fn new(deps: ChunkingEvaluationServiceDeps) -> Arc<Self> {
        Arc::new(Self {
            blog_source: deps.blog_source,
            generator: deps.generator,
            embedding_service: deps.embedding_service,
            settings: deps.settings,
            evaluation_dataset_store: deps.evaluation_dataset_store,
            evaluation_result_store: deps.evaluation_result_store,
            post_chunking_service: deps.post_chunking_service,
            tokenizer: deps.tokenizer,
        })
    }

    pub async fn get_dataset_status(
        &self,
        slug: &str,
    ) -> Result<EvaluationDatasetStatus, AppError> {
        let post = Post::try_new(self.blog_source.fetch(slug).await?)?;
        self.evaluation_dataset_store
            .status(slug, post.version().as_str())
            .await
    }

    pub async fn get_latest_result(
        &self,
        slug: &str,
    ) -> Result<Option<EvaluationRunResult>, AppError> {
        let post = Post::try_new(self.blog_source.fetch(slug).await?)?;
        self.evaluation_result_store
            .load(post.slug(), post.version().as_str())
            .await
    }

    pub async fn get_result_history(
        &self,
        slug: &str,
    ) -> Result<Vec<EvaluationRunSummary>, AppError> {
        let post = Post::try_new(self.blog_source.fetch(slug).await?)?;
        self.evaluation_result_store
            .list(post.slug(), post.version().as_str())
            .await
    }

    pub async fn get_result_run_by_id(
        &self,
        slug: &str,
        run_id: &str,
    ) -> Result<Option<EvaluationRunResult>, AppError> {
        let post = Post::try_new(self.blog_source.fetch(slug).await?)?;
        self.evaluation_result_store
            .load_run(post.slug(), post.version().as_str(), run_id)
            .await
    }

    pub async fn run_evaluation(
        &self,
        slug: &str,
        variants: Vec<ChunkingVariant>,
        options: EvaluationRunOptions,
        job: Option<Arc<Job>>,
    ) -> Result<EvaluationRunResult, AppError> {
        let progress = job.map(|job| job as Arc<dyn EvaluationProgress>);
        self.run_evaluation_use_case()
            .execute(slug, variants, options, progress, true)
            .await
    }

    pub async fn run_matrix_evaluation(
        &self,
        slug: &str,
        variant: ChunkingVariant,
        option_sets: Vec<EvaluationRunOptions>,
        job: Arc<Job>,
    ) -> Result<(), AppError> {
        let progress: Arc<dyn EvaluationProgress> = job;
        self.run_matrix_evaluation_use_case()
            .execute(slug, variant, option_sets, progress)
            .await
    }

    pub async fn run_autotune_evaluation(
        &self,
        slug: &str,
        request: EvaluationAutotuneRequest,
        job: Arc<Job>,
    ) -> Result<(), AppError> {
        let progress: Arc<dyn EvaluationProgress> = job;
        self.run_autotune_evaluation_use_case()
            .execute(slug, request, progress)
            .await
    }

    pub async fn generate_dataset(&self, slug: &str, job: Arc<Job>) -> Result<(), AppError> {
        GenerateSyntheticDatasetUseCase::new(
            self.blog_source.clone(),
            self.generator.clone(),
            self.embedding_service.clone(),
            self.settings.clone(),
            self.evaluation_dataset_store.clone(),
        )
        .execute(slug, job)
        .await
    }

    fn run_evaluation_use_case(&self) -> RunEvaluationUseCase {
        RunEvaluationUseCase::new(
            self.blog_source.clone(),
            self.embedding_service.clone(),
            self.settings.clone(),
            self.post_chunking_service.clone(),
            self.tokenizer.clone(),
            self.evaluation_dataset_store.clone(),
            self.evaluation_result_store.clone(),
        )
    }

    fn run_matrix_evaluation_use_case(&self) -> RunMatrixEvaluationUseCase {
        RunMatrixEvaluationUseCase::new(
            self.run_evaluation_use_case(),
            self.evaluation_result_store.clone(),
        )
    }

    fn run_autotune_evaluation_use_case(&self) -> RunAutotuneEvaluationUseCase {
        RunAutotuneEvaluationUseCase::new(
            self.run_evaluation_use_case(),
            self.evaluation_result_store.clone(),
        )
    }
}
