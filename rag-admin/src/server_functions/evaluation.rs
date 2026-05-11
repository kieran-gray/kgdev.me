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
            status: d.status.as_str().to_string(),
            created_at: d.created_at.to_string(),
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
            document_version: d.document_version,
            content_hash: d.content_hash,
            label: d.label,
            status: d.status.as_str().to_string(),
            target_question_count: d.target_question_count,
            question_count: d.question_count,
            rejection_count: d.rejection_count,
            generation_model_id: d.generation_model_id,
            generation_model: d.generation_model,
            embedding_model_id: d.embedding_model_id,
            failure_reason: d.failure_reason,
            questions: questions.into_iter().map(|q| q.into()).collect(),
            created_at: d.created_at.to_string(),
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
    pipeline_configuration_id: Uuid,
    label: String,
) -> Result<EvaluationJobInfo, ServerFnError> {
    use crate::server::application::ports::{Clock, IdGenerator};
    use crate::server::domain::evaluation::dataset::commands::{
        EvaluationDatasetCommand, RequestDatasetGeneration,
    };
    use crate::server::infrastructure::id::UuidGenerator;
    use crate::server::infrastructure::time::SystemClock;
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    let eval_settings = state
        .evaluation_defaults_store
        .load()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .evaluation;

    let pipeline = state
        .pipeline_resolver
        .resolve(pipeline_configuration_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let document = state
        .source_document_query_service
        .get_detail(document_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new(format!("document {document_id} not found")))?
        .document;

    let dataset_id = UuidGenerator.new_uuid();
    let occurred_at = SystemClock.now();

    state
        .evaluation_dataset_command_processor
        .handle(
            dataset_id,
            EvaluationDatasetCommand::RequestDatasetGeneration(RequestDatasetGeneration {
                dataset_id,
                document_id,
                document_version: document.latest_version,
                content_hash: document.latest_content_hash.clone(),
                label,
                target_question_count: eval_settings.question_count,
                generation_model_id: pipeline.generation_model.generation_model_id,
                generation_model: pipeline.generation_model.model.clone(),
                excerpt_similarity_threshold_milli: eval_settings
                    .excerpt_similarity_threshold_milli,
                duplicate_similarity_threshold_milli: eval_settings
                    .duplicate_similarity_threshold_milli,
                embedding_model_id: pipeline.embedding_model.embedding_model_id,
                occurred_at,
            }),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(EvaluationJobInfo {
        job_id: dataset_id.to_string(),
        stream_url: format!("/api/events/ws?stream_id={dataset_id}"),
    })
}

#[server(name = RenameDataset, prefix = "/api", endpoint = "rename_dataset")]
pub async fn rename_dataset(dataset_id: Uuid, label: String) -> Result<(), ServerFnError> {
    use crate::server::application::ports::Clock;
    use crate::server::domain::evaluation::dataset::commands::{
        EvaluationDatasetCommand, RenameDataset,
    };
    use crate::server::infrastructure::time::SystemClock;
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .evaluation_dataset_command_processor
        .handle(
            dataset_id,
            EvaluationDatasetCommand::RenameDataset(RenameDataset {
                dataset_id,
                label,
                occurred_at: SystemClock.now(),
            }),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(name = DeleteDataset, prefix = "/api", endpoint = "delete_dataset")]
pub async fn delete_dataset(dataset_id: Uuid) -> Result<(), ServerFnError> {
    use crate::server::application::ports::Clock;
    use crate::server::domain::evaluation::dataset::commands::{
        DeleteDataset, EvaluationDatasetCommand,
    };
    use crate::server::infrastructure::time::SystemClock;
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    state
        .evaluation_dataset_command_processor
        .handle(
            dataset_id,
            EvaluationDatasetCommand::DeleteDataset(DeleteDataset {
                dataset_id,
                occurred_at: SystemClock.now(),
            }),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(
    name = StartRunEvaluation,
    prefix = "/api",
    endpoint = "start_run_evaluation"
)]
pub async fn start_run_evaluation(
    request: RunEvaluationRequestDto,
) -> Result<EvaluationJobInfo, ServerFnError> {
    use crate::server::application::ports::Clock;
    use crate::server::domain::evaluation::run::aggregate::EvaluationRun;
    use crate::server::domain::evaluation::run::commands::{EvaluationRunCommand, RequestRun};
    use crate::server::domain::evaluation::run::scoring_policy::ScoringPolicy;
    use crate::server::infrastructure::time::SystemClock;
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;

    let dataset = state
        .evaluation_query_service
        .get_dataset(request.dataset_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| {
            ServerFnError::new(format!(
                "evaluation dataset {} not found",
                request.dataset_id
            ))
        })?;

    let scoring_policy = ScoringPolicy::default();
    let run_id = EvaluationRun::compute_id(
        request.dataset_id,
        request.pipeline_configuration_id,
        &request.variants,
        &request.options,
        request.autotune.as_ref(),
    );
    let occurred_at = SystemClock.now();

    state
        .evaluation_run_command_processor
        .handle(
            run_id,
            EvaluationRunCommand::RequestRun(RequestRun {
                run_id,
                dataset_id: request.dataset_id,
                pipeline_configuration_id: request.pipeline_configuration_id,
                document_id: dataset.document_id,
                document_version: dataset.document_version,
                variants: request.variants,
                options: request.options,
                autotune_request: request.autotune,
                scoring_policy,
                occurred_at,
            }),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(EvaluationJobInfo {
        job_id: run_id.to_string(),
        stream_url: format!("/api/events/ws?stream_id={run_id}"),
    })
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
            status: r.status.as_str().to_string(),
            variant_count: r.variants_count,
            created_at: r.created_at.to_string(),
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
        status: r.status.as_str().to_string(),
        variants: r.variant_results.into_iter().map(|v| v.into()).collect(),
        created_at: r.created_at.to_string(),
    }))
}
