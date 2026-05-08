use std::{collections::HashSet, sync::Arc};

use futures_util::lock::Mutex;

use crate::{
    server::application::{
        evaluation::{progress::EvaluationProgress, ChunkingEvaluationService},
        AppError, JobRegistry,
    },
    shared::{ChunkingVariant, EvaluationAutotuneRequest, EvaluationJobInfo, EvaluationRunOptions},
};

pub struct EvaluationJobService {
    pub job_registry: Arc<JobRegistry>,
    pub running: Mutex<HashSet<String>>,
    pub evaluation_service: Arc<ChunkingEvaluationService>,
}

impl EvaluationJobService {
    pub fn new(
        job_registry: Arc<JobRegistry>,
        evaluation_service: Arc<ChunkingEvaluationService>,
    ) -> Arc<Self> {
        Arc::new(Self {
            job_registry,
            running: Mutex::new(HashSet::new()),
            evaluation_service,
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
                .evaluation_service
                .generate_dataset(&slug_for_task, job_for_task.clone())
                .await;
            if let Err(e) = result {
                job.error(format!("chunking evaluation generation failed: {e}"))
                    .await;
            }
            job_for_task.finish().await;
            svc.running.lock().await.remove(&slug_for_task);
        });

        Ok(EvaluationJobInfo { job_id, stream_url })
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
                .evaluation_service
                .run_evaluation(&slug, variants, options, Some(job.clone()))
                .await;
            if let Err(e) = result {
                job.error(format!("evaluation failed: {e}")).await;
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
                .evaluation_service
                .run_matrix_evaluation(&slug, variant, option_sets, job.clone())
                .await
            {
                job.error(format!("evaluation failed: {e}")).await;
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
                .evaluation_service
                .run_autotune_evaluation(&slug, request, job.clone())
                .await
            {
                job.error(format!("autotune failed: {e}")).await;
            }
            job.finish().await;
        });

        Ok(EvaluationJobInfo { job_id, stream_url })
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
