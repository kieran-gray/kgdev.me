use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::server::domain::evaluation::run::{
    read_model::{EvaluationRunReadModel, EvaluationVariantResultDto},
    repository::{EvaluationRunRepository, EvaluationRunRepositoryError},
};

pub struct PostgresEvaluationRunRepository {
    pool: PgPool,
}

impl PostgresEvaluationRunRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EvaluationRunRepository for PostgresEvaluationRunRepository {
    async fn load(
        &self,
        run_id: Uuid,
    ) -> Result<Option<EvaluationRunReadModel>, EvaluationRunRepositoryError> {
        let row: Option<RunRow> = sqlx::query_as(
            r#"
            SELECT 
                run_id, dataset_id, pipeline_configuration_id, document_id, document_version,
                status, variants_count, variants_prepared, variants_scored, failure_reason,
                scoring_recall_weight, scoring_iou_weight, scoring_precision_weight,
                scoring_precision_omega_weight, created_at
            FROM evaluation_runs 
            WHERE run_id = $1
            "#,
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| EvaluationRunRepositoryError::Internal(format!("load: {e}")))?;

        match row {
            None => Ok(None),
            Some(row) => {
                let mut read_model = EvaluationRunReadModel::from(row);
                read_model.variant_results = self.load_variant_results(run_id).await?;
                Ok(Some(read_model))
            }
        }
    }

    async fn save(
        &self,
        read_model: EvaluationRunReadModel,
    ) -> Result<(), EvaluationRunRepositoryError> {
        let status = serde_json::to_string(&read_model.status).map_err(|e| {
            EvaluationRunRepositoryError::Internal(format!("serialize status: {e}"))
        })?;

        sqlx::query(
            r#"
            INSERT INTO evaluation_runs (
                run_id, dataset_id, pipeline_configuration_id, document_id, document_version,
                status, variants_count, variants_prepared, variants_scored, failure_reason,
                scoring_recall_weight, scoring_iou_weight, scoring_precision_weight,
                scoring_precision_omega_weight, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, NOW())
            ON CONFLICT (run_id) DO UPDATE SET
                status = $6,
                variants_prepared = $8,
                variants_scored = $9,
                failure_reason = $10,
                updated_at = NOW()
            "#,
        )
        .bind(read_model.run_id)
        .bind(read_model.dataset_id)
        .bind(read_model.pipeline_configuration_id)
        .bind(read_model.document_id)
        .bind(read_model.document_version as i32)
        .bind(&status)
        .bind(read_model.variants_count as i32)
        .bind(read_model.variants_prepared as i32)
        .bind(read_model.variants_scored as i32)
        .bind(&read_model.failure_reason)
        .bind(read_model.scoring_recall_weight)
        .bind(read_model.scoring_iou_weight)
        .bind(read_model.scoring_precision_weight)
        .bind(read_model.scoring_precision_omega_weight)
        .bind(
            chrono::DateTime::parse_from_rfc3339(&read_model.created_at)
                .map_err(|e| {
                    EvaluationRunRepositoryError::Internal(format!("parse created_at: {e}"))
                })?
                .with_timezone(&chrono::Utc),
        )
        .execute(&self.pool)
        .await
        .map_err(|e| EvaluationRunRepositoryError::Internal(format!("save: {e}")))?;

        Ok(())
    }

    async fn save_variant_result(
        &self,
        result: EvaluationVariantResultDto,
    ) -> Result<(), EvaluationRunRepositoryError> {
        let split = serde_json::to_string(&result.split)
            .map_err(|e| EvaluationRunRepositoryError::Internal(format!("serialize split: {e}")))?;

        let mut tx = self.pool.begin().await.map_err(|e| {
            EvaluationRunRepositoryError::Internal(format!("begin transaction: {e}"))
        })?;

        sqlx::query(
            r#"
            INSERT INTO evaluation_variant_results (
                run_id, variant_label, split, recall_mean, recall_std,
                precision_mean, precision_std, iou_mean, iou_std,
                precision_omega_mean, precision_omega_std,
                chunk_set_id, embedding_set_id, selected
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (run_id, variant_label, split) DO UPDATE SET
                recall_mean = $4, recall_std = $5,
                precision_mean = $6, precision_std = $7,
                iou_mean = $8, iou_std = $9,
                precision_omega_mean = $10, precision_omega_std = $11,
                selected = $14
            "#,
        )
        .bind(result.run_id)
        .bind(&result.variant_label)
        .bind(&split)
        .bind(result.recall_mean)
        .bind(result.recall_std)
        .bind(result.precision_mean)
        .bind(result.precision_std)
        .bind(result.iou_mean)
        .bind(result.iou_std)
        .bind(result.precision_omega_mean)
        .bind(result.precision_omega_std)
        .bind(result.chunk_set_id)
        .bind(result.embedding_set_id)
        .bind(result.selected)
        .execute(&mut *tx)
        .await
        .map_err(|e| EvaluationRunRepositoryError::Internal(format!("save_variant_result: {e}")))?;

        for trace in &result.retrieval_traces {
            let retrieved_chunk_ids =
                serde_json::to_value(&trace.retrieved_chunk_ids).map_err(|e| {
                    EvaluationRunRepositoryError::Internal(format!(
                        "serialize retrieved_chunk_ids: {e}"
                    ))
                })?;
            let scores = serde_json::to_value(&trace.scores).map_err(|e| {
                EvaluationRunRepositoryError::Internal(format!("serialize scores: {e}"))
            })?;

            sqlx::query(
                r#"
                INSERT INTO retrieval_traces (
                    run_id, variant_label, split, question_sequence,
                    retrieved_chunk_ids, scores, recall, precision, iou
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (run_id, variant_label, split, question_sequence) DO UPDATE SET
                    retrieved_chunk_ids = $5,
                    scores = $6,
                    recall = $7,
                    precision = $8,
                    iou = $9
                "#,
            )
            .bind(result.run_id)
            .bind(&result.variant_label)
            .bind(&split)
            .bind(trace.question_sequence as i32)
            .bind(&retrieved_chunk_ids)
            .bind(&scores)
            .bind(trace.recall)
            .bind(trace.precision)
            .bind(trace.iou)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                EvaluationRunRepositoryError::Internal(format!("save_retrieval_trace: {e}"))
            })?;
        }

        tx.commit().await.map_err(|e| {
            EvaluationRunRepositoryError::Internal(format!("commit transaction: {e}"))
        })?;

        Ok(())
    }

    async fn list_for_document(
        &self,
        document_id: Uuid,
    ) -> Result<Vec<EvaluationRunReadModel>, EvaluationRunRepositoryError> {
        let rows: Vec<RunRow> = sqlx::query_as(
            r#"
            SELECT 
                run_id, dataset_id, pipeline_configuration_id, document_id, document_version,
                status, variants_count, variants_prepared, variants_scored, failure_reason,
                scoring_recall_weight, scoring_iou_weight, scoring_precision_weight,
                scoring_precision_omega_weight, created_at
            FROM evaluation_runs 
            WHERE document_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| EvaluationRunRepositoryError::Internal(format!("list_for_document: {e}")))?;

        Ok(rows.into_iter().map(EvaluationRunReadModel::from).collect())
    }

    async fn list_for_dataset(
        &self,
        dataset_id: Uuid,
    ) -> Result<Vec<EvaluationRunReadModel>, EvaluationRunRepositoryError> {
        let rows: Vec<RunRow> = sqlx::query_as(
            r#"
            SELECT 
                run_id, dataset_id, pipeline_configuration_id, document_id, document_version,
                status, variants_count, variants_prepared, variants_scored, failure_reason,
                scoring_recall_weight, scoring_iou_weight, scoring_precision_weight,
                scoring_precision_omega_weight, created_at
            FROM evaluation_runs 
            WHERE dataset_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(dataset_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| EvaluationRunRepositoryError::Internal(format!("list_for_dataset: {e}")))?;

        Ok(rows.into_iter().map(EvaluationRunReadModel::from).collect())
    }

    async fn load_variant_results(
        &self,
        run_id: Uuid,
    ) -> Result<Vec<EvaluationVariantResultDto>, EvaluationRunRepositoryError> {
        let rows: Vec<VariantResultRow> = sqlx::query_as(
            r#"
            SELECT 
                run_id, variant_label, split, recall_mean, recall_std,
                precision_mean, precision_std, iou_mean, iou_std,
                precision_omega_mean, precision_omega_std,
                chunk_set_id, embedding_set_id, selected
            FROM evaluation_variant_results 
            WHERE run_id = $1
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            EvaluationRunRepositoryError::Internal(format!("load_variant_results: {e}"))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let trace_rows: Vec<RetrievalTraceRow> = sqlx::query_as(
                r#"
                SELECT 
                    question_sequence, retrieved_chunk_ids, scores, recall, precision, iou
                FROM retrieval_traces 
                WHERE run_id = $1 AND variant_label = $2 AND split = $3
                ORDER BY question_sequence ASC
                "#,
            )
            .bind(run_id)
            .bind(&row.variant_label)
            .bind(&row.split)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                EvaluationRunRepositoryError::Internal(format!("load_retrieval_traces: {e}"))
            })?;

            results.push(EvaluationVariantResultDto {
                run_id: row.run_id,
                variant_label: row.variant_label,
                split: serde_json::from_str(&row.split)
                    .unwrap_or(crate::shared::EvaluationResultSplit::Full),
                recall_mean: row.recall_mean,
                recall_std: row.recall_std,
                precision_mean: row.precision_mean,
                precision_std: row.precision_std,
                iou_mean: row.iou_mean,
                iou_std: row.iou_std,
                precision_omega_mean: row.precision_omega_mean,
                precision_omega_std: row.precision_omega_std,
                chunk_set_id: row.chunk_set_id,
                embedding_set_id: row.embedding_set_id,
                selected: row.selected,
                retrieval_traces: trace_rows.into_iter().map(|r| r.into()).collect(),
            });
        }

        Ok(results)
    }
}

#[derive(sqlx::FromRow)]
struct RunRow {
    run_id: Uuid,
    dataset_id: Uuid,
    pipeline_configuration_id: Uuid,
    document_id: Uuid,
    document_version: i32,
    status: String,
    variants_count: i32,
    variants_prepared: i32,
    variants_scored: i32,
    failure_reason: Option<String>,
    scoring_recall_weight: f32,
    scoring_iou_weight: f32,
    scoring_precision_weight: f32,
    scoring_precision_omega_weight: f32,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<RunRow> for EvaluationRunReadModel {
    fn from(row: RunRow) -> Self {
        Self {
            run_id: row.run_id,
            dataset_id: row.dataset_id,
            pipeline_configuration_id: row.pipeline_configuration_id,
            document_id: row.document_id,
            document_version: row.document_version as u32,
            status: serde_json::from_str(&row.status).unwrap_or(
                crate::server::domain::evaluation::run::aggregate::EvaluationRunStatus::Pending,
            ),
            variants_count: row.variants_count as u32,
            variants_prepared: row.variants_prepared as u32,
            variants_scored: row.variants_scored as u32,
            failure_reason: row.failure_reason,
            scoring_recall_weight: row.scoring_recall_weight,
            scoring_iou_weight: row.scoring_iou_weight,
            scoring_precision_weight: row.scoring_precision_weight,
            scoring_precision_omega_weight: row.scoring_precision_omega_weight,
            created_at: row.created_at.to_rfc3339(),
            variant_results: Vec::new(),
        }
    }
}

#[derive(sqlx::FromRow)]
struct VariantResultRow {
    run_id: Uuid,
    variant_label: String,
    split: String,
    recall_mean: f32,
    recall_std: f32,
    precision_mean: f32,
    precision_std: f32,
    iou_mean: f32,
    iou_std: f32,
    precision_omega_mean: f32,
    precision_omega_std: f32,
    chunk_set_id: Uuid,
    embedding_set_id: Uuid,
    selected: bool,
}

#[derive(sqlx::FromRow)]
struct RetrievalTraceRow {
    question_sequence: i32,
    retrieved_chunk_ids: serde_json::Value,
    scores: serde_json::Value,
    recall: f32,
    precision: f32,
    iou: f32,
}

impl From<RetrievalTraceRow>
    for crate::server::domain::evaluation::run::events::RetrievalTraceEntry
{
    fn from(row: RetrievalTraceRow) -> Self {
        Self {
            question_sequence: row.question_sequence as u32,
            retrieved_chunk_ids: serde_json::from_value(row.retrieved_chunk_ids)
                .unwrap_or_default(),
            scores: serde_json::from_value(row.scores).unwrap_or_default(),
            recall: row.recall,
            precision: row.precision,
            iou: row.iou,
        }
    }
}
