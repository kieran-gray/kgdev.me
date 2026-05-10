use std::{collections::HashSet, sync::Arc};

use futures_util::lock::Mutex;
use uuid::Uuid;

use crate::{
    server::application::{
        evaluation::{
            progress::EvaluationProgress,
            use_cases::{
                evaluation_dataset::{
                    GenerateEvaluationDatasetRequest, GenerateSyntheticDatasetUseCase,
                },
                evaluation_run::{RunEvaluationRequest, RunEvaluationUseCase},
            },
        },
        AppError, JobRegistry,
    },
    shared::{ChunkingVariant, EvaluationAutotuneRequest, EvaluationJobInfo, EvaluationRunOptions},
};

pub struct EvaluationJobService {
    pub job_registry: Arc<JobRegistry>,
    pub running: Mutex<HashSet<Uuid>>,
    pub generate_synthetic_dataset_use_case: Arc<GenerateSyntheticDatasetUseCase>,
    pub run_evaluation_use_case: Arc<RunEvaluationUseCase>,
}

impl EvaluationJobService {
    pub fn new(
        job_registry: Arc<JobRegistry>,
        generate_synthetic_dataset_use_case: Arc<GenerateSyntheticDatasetUseCase>,
        run_evaluation_use_case: Arc<RunEvaluationUseCase>,
    ) -> Arc<Self> {
        Arc::new(Self {
            job_registry,
            running: Mutex::new(HashSet::new()),
            generate_synthetic_dataset_use_case,
            run_evaluation_use_case,
        })
    }

    pub async fn start_generate_dataset(
        self: &Arc<Self>,
        request: GenerateEvaluationDatasetRequest,
    ) -> Result<EvaluationJobInfo, AppError> {
        {
            let mut guard = self.running.lock().await;
            if guard.contains(&request.document_id) {
                return Err(AppError::Validation(format!(
                    "evaluation dataset generation for {} is already running",
                    request.document_id
                )));
            }
            guard.insert(request.document_id);
        }

        let (job_id, job) = self.job_registry.create().await;
        let stream_url = format!("/api/ingest/logs/{job_id}");

        let svc = self.clone();
        let doc_id = request.document_id;
        tokio::spawn(async move {
            let result = svc
                .generate_synthetic_dataset_use_case
                .execute(request, job.clone())
                .await;
            if let Err(e) = result {
                job.error(format!("evaluation dataset generation failed: {e}"))
                    .await;
            }
            job.finish().await;
            svc.running.lock().await.remove(&doc_id);
        });

        Ok(EvaluationJobInfo { job_id, stream_url })
    }

    pub async fn start_run_evaluation(
        self: &Arc<Self>,
        request: RunEvaluationRequest,
    ) -> Result<EvaluationJobInfo, AppError> {
        let (job_id, job) = self.job_registry.create().await;
        let stream_url = format!("/api/ingest/logs/{job_id}");

        let svc = self.clone();
        tokio::spawn(async move {
            let result = svc
                .run_evaluation_use_case
                .execute(request, Some(job.clone()))
                .await;
            if let Err(e) = result {
                job.error(format!("evaluation run failed: {e}")).await;
            }
            job.finish().await;
        });

        Ok(EvaluationJobInfo { job_id, stream_url })
    }

    // Matrix and Autotune are temporarily handled by calling run_evaluation repeatedly
    // or we can add them as separate use cases if they are already rewritten.
    // For now, let's keep it simple.

    pub async fn start_run_evaluation_matrix(
        self: &Arc<Self>,
        slug: String,
        variant: ChunkingVariant,
        option_sets: Vec<EvaluationRunOptions>,
    ) -> Result<EvaluationJobInfo, AppError> {
        // TODO: Implement or route to RunMatrixEvaluationUseCase
        Err(AppError::Internal(
            "Not implemented in the new architecture yet".into(),
        ))
    }

    pub async fn start_run_evaluation_autotune(
        self: &Arc<Self>,
        slug: String,
        request: EvaluationAutotuneRequest,
    ) -> Result<EvaluationJobInfo, AppError> {
        // TODO: Implement or route to RunAutotuneEvaluationUseCase
        Err(AppError::Internal(
            "Not implemented in the new architecture yet".into(),
        ))
    }
}
