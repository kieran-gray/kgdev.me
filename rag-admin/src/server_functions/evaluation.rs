use leptos::prelude::*;
use uuid::Uuid;

use crate::shared::{
    EvaluationDatasetDto, EvaluationDatasetSummaryDto, EvaluationJobInfo, EvaluationRunDto,
    EvaluationRunSummaryDto, RunEvaluationRequestDto,
};

#[server(
    name = GetDatasetsForDocument,
    prefix = "/api",
    endpoint = "get_datasets_for_document"
)]
pub async fn get_datasets_for_document(
    document_id: Uuid,
) -> Result<Vec<EvaluationDatasetSummaryDto>, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    let datasets = state
        .evaluation_query_service
        .list_datasets_for_document(document_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(datasets
        .into_iter()
        .map(|d| EvaluationDatasetSummaryDto {
            dataset_id: d.dataset_id,
            label: d.label,
            question_count: d.question_count,
            status: format!("{:?}", d.status),
            created_at: d.created_at,
        })
        .collect())
}

#[server(name = GetDataset, prefix = "/api", endpoint = "get_dataset")]
pub async fn get_dataset(dataset_id: Uuid) -> Result<Option<EvaluationDatasetDto>, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    let dataset = state
        .evaluation_query_service
        .get_dataset(dataset_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if let Some(d) = dataset {
        let questions = state
            .evaluation_query_service
            .load_questions(dataset_id)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(Some(EvaluationDatasetDto {
            dataset_id: d.dataset_id,
            document_id: d.document_id,
            label: d.label,
            status: format!("{:?}", d.status),
            questions: questions.into_iter().map(|q| q.into()).collect(),
            created_at: d.created_at,
        }))
    } else {
        Ok(None)
    }
}

#[server(
    name = StartGenerateSyntheticDataset,
    prefix = "/api",
    endpoint = "start_generate_synthetic_dataset"
)]
pub async fn start_generate_synthetic_dataset(
    document_id: Uuid,
    label: String,
) -> Result<EvaluationJobInfo, ServerFnError> {
    use crate::server::application::evaluation::use_cases::evaluation_dataset::GenerateEvaluationDatasetRequest;
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    let settings = state.settings.read().await.clone();
    let eval_settings = settings.evaluation;

    let request = GenerateEvaluationDatasetRequest {
        document_id,
        label,
        embedding_model_id: settings.embedding_model.id.parse().unwrap_or_default(), // FIXME
        generation_model: eval_settings.generation_model.clone(),
        generation_backend: eval_settings.generation_backend.as_str().to_string(),
        target_question_count: eval_settings.question_count,
        excerpt_similarity_threshold_milli: eval_settings.excerpt_similarity_threshold_milli,
        duplicate_similarity_threshold_milli: eval_settings.duplicate_similarity_threshold_milli,
    };

    state
        .evaluation_job_service
        .start_generate_dataset(request)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(
    name = StartRunEvaluation,
    prefix = "/api",
    endpoint = "start_run_evaluation"
)]
pub async fn start_run_evaluation(
    request: RunEvaluationRequestDto,
) -> Result<EvaluationJobInfo, ServerFnError> {
    use crate::server::application::evaluation::use_cases::evaluation_run::RunEvaluationRequest;
    use crate::server::domain::evaluation::run::scoring_policy::ScoringPolicy;
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    let app_request = RunEvaluationRequest {
        dataset_id: request.dataset_id,
        pipeline_configuration_id: request.pipeline_configuration_id,
        variants: request.variants,
        options: request.options,
        autotune_request: None,
        scoring_policy: ScoringPolicy::default(),
    };

    state
        .evaluation_job_service
        .start_run_evaluation(app_request)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(
    name = GetRunsForDocument,
    prefix = "/api",
    endpoint = "get_runs_for_document"
)]
pub async fn get_runs_for_document(
    document_id: Uuid,
) -> Result<Vec<EvaluationRunSummaryDto>, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    let runs = state
        .evaluation_query_service
        .list_runs_for_document(document_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(runs
        .into_iter()
        .map(|r| EvaluationRunSummaryDto {
            run_id: r.run_id,
            dataset_id: r.dataset_id,
            status: format!("{:?}", r.status),
            variant_count: r.variants_count,
            created_at: r.created_at,
        })
        .collect())
}

#[server(name = GetRun, prefix = "/api", endpoint = "get_run")]
pub async fn get_run(run_id: Uuid) -> Result<Option<EvaluationRunDto>, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    let run = state
        .evaluation_query_service
        .get_run(run_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(run.map(|r| EvaluationRunDto {
        run_id: r.run_id,
        dataset_id: r.dataset_id,
        status: format!("{:?}", r.status),
        variants: r.variant_results.into_iter().map(|v| v.into()).collect(),
        created_at: r.created_at,
    }))
}
