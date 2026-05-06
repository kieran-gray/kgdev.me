use leptos::prelude::*;

use crate::shared::{
    ChunkingVariant, EvaluationAutotuneRequest, EvaluationDatasetStatus, EvaluationJobInfo,
    EvaluationRunOptions, EvaluationRunResult, EvaluationRunSummary,
};

#[server(
    name = GetEvaluationDatasetStatus,
    prefix = "/api",
    endpoint = "get_evaluation_dataset_status"
)]
pub async fn get_evaluation_dataset_status(
    slug: String,
) -> Result<EvaluationDatasetStatus, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .chunking_evaluation_service
        .dataset_status(&slug)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(
    name = StartGenerateEvaluationDataset,
    prefix = "/api",
    endpoint = "start_generate_evaluation_dataset"
)]
pub async fn start_generate_evaluation_dataset(
    slug: String,
) -> Result<EvaluationJobInfo, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .chunking_evaluation_service
        .start_generate_dataset(slug)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(
    name = RunChunkingEvaluation,
    prefix = "/api",
    endpoint = "run_chunking_evaluation"
)]
pub async fn run_chunking_evaluation(
    slug: String,
    variants: Vec<ChunkingVariant>,
    options: EvaluationRunOptions,
) -> Result<EvaluationRunResult, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .chunking_evaluation_service
        .run_evaluation(&slug, variants, options, None)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(
    name = GetLatestEvaluationResult,
    prefix = "/api",
    endpoint = "get_latest_evaluation_result"
)]
pub async fn get_latest_evaluation_result(
    slug: String,
) -> Result<Option<EvaluationRunResult>, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .chunking_evaluation_service
        .latest_result(&slug)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(
    name = GetEvaluationResultHistory,
    prefix = "/api",
    endpoint = "get_evaluation_result_history"
)]
pub async fn get_evaluation_result_history(
    slug: String,
) -> Result<Vec<EvaluationRunSummary>, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .chunking_evaluation_service
        .result_history(&slug)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(
    name = GetEvaluationResultRun,
    prefix = "/api",
    endpoint = "get_evaluation_result_run"
)]
pub async fn get_evaluation_result_run(
    slug: String,
    run_id: String,
) -> Result<Option<EvaluationRunResult>, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .chunking_evaluation_service
        .result_run(&slug, &run_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(
    name = StartRunEvaluation,
    prefix = "/api",
    endpoint = "start_run_evaluation"
)]
pub async fn start_run_evaluation(
    slug: String,
    variants: Vec<ChunkingVariant>,
    options: EvaluationRunOptions,
) -> Result<EvaluationJobInfo, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .chunking_evaluation_service
        .start_run_evaluation(slug, variants, options)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(
    name = StartRunEvaluationMatrix,
    prefix = "/api",
    endpoint = "start_run_evaluation_matrix"
)]
pub async fn start_run_evaluation_matrix(
    slug: String,
    variant: ChunkingVariant,
    option_sets: Vec<EvaluationRunOptions>,
) -> Result<EvaluationJobInfo, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .chunking_evaluation_service
        .start_run_evaluation_matrix(slug, variant, option_sets)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(
    name = StartRunEvaluationAutotune,
    prefix = "/api",
    endpoint = "start_run_evaluation_autotune"
)]
pub async fn start_run_evaluation_autotune(
    slug: String,
    request: EvaluationAutotuneRequest,
) -> Result<EvaluationJobInfo, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    state
        .chunking_evaluation_service
        .start_run_evaluation_autotune(slug, request)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
