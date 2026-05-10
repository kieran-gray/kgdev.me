use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::server::{application::ports::Clock, domain::evaluation::{
    dataset::{
        read_model::EvaluationDatasetReadModel,
        repository::{EvaluationDatasetRepository, EvaluationDatasetRepositoryError},
    },
    question::EvaluationQuestion,
}};

pub struct PostgresEvaluationDatasetRepository {
    pool: PgPool,
    clock: Arc<dyn Clock>
}

impl PostgresEvaluationDatasetRepository {
    pub fn new(pool: PgPool, clock: Arc<dyn Clock>) -> Self {
        Self { pool, clock }
    }
}

#[async_trait]
impl EvaluationDatasetRepository for PostgresEvaluationDatasetRepository {
    async fn load(
        &self,
        dataset_id: Uuid,
    ) -> Result<Option<EvaluationDatasetReadModel>, EvaluationDatasetRepositoryError> {
        let row: Option<DatasetRow> = sqlx::query_as(
            r#"
            SELECT 
                dataset_id, document_id, document_version, content_hash, label,
                target_question_count, generation_model, generation_backend,
                excerpt_similarity_threshold_milli, duplicate_similarity_threshold_milli,
                embedding_model_id, status, question_count, rejection_count,
                failure_reason, created_at
            FROM evaluation_datasets 
            WHERE dataset_id = $1
            "#,
        )
        .bind(dataset_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| EvaluationDatasetRepositoryError::Internal(format!("load: {e}")))?;

        Ok(row.map(EvaluationDatasetReadModel::from))
    }

    async fn save(
        &self,
        read_model: EvaluationDatasetReadModel,
    ) -> Result<(), EvaluationDatasetRepositoryError> {
        let status = serde_json::to_string(&read_model.status).map_err(|e| {
            EvaluationDatasetRepositoryError::Internal(format!("serialize status: {e}"))
        })?;

        sqlx::query(
            r#"
            INSERT INTO evaluation_datasets (
                dataset_id, document_id, document_version, content_hash, label,
                target_question_count, generation_model, generation_backend,
                excerpt_similarity_threshold_milli, duplicate_similarity_threshold_milli,
                embedding_model_id, status, question_count, rejection_count,
                failure_reason, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, NOW())
            ON CONFLICT (dataset_id) DO UPDATE SET
                status = $12,
                question_count = $13,
                rejection_count = $14,
                failure_reason = $15,
                updated_at = NOW()
            "#,
        )
        .bind(read_model.dataset_id)
        .bind(read_model.document_id)
        .bind(read_model.document_version as i32)
        .bind(&read_model.content_hash)
        .bind(&read_model.label)
        .bind(read_model.target_question_count as i32)
        .bind(&read_model.generation_model)
        .bind(&read_model.generation_backend)
        .bind(read_model.excerpt_similarity_threshold_milli as i32)
        .bind(read_model.duplicate_similarity_threshold_milli as i32)
        .bind(read_model.embedding_model_id)
        .bind(&status)
        .bind(read_model.question_count as i32)
        .bind(read_model.rejection_count as i32)
        .bind(&read_model.failure_reason)
        .bind(
            chrono::DateTime::parse_from_rfc3339(&read_model.created_at)
                .map_err(|e| {
                    EvaluationDatasetRepositoryError::Internal(format!("parse created_at: {e}"))
                })?
                .with_timezone(&chrono::Utc),
        )
        .execute(&self.pool)
        .await
        .map_err(|e| EvaluationDatasetRepositoryError::Internal(format!("save: {e}")))?;

        Ok(())
    }

    async fn save_question(
        &self,
        dataset_id: Uuid,
        question: EvaluationQuestion,
    ) -> Result<(), EvaluationDatasetRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            EvaluationDatasetRepositoryError::Internal(format!("begin transaction: {e}"))
        })?;

        let embedding = serde_json::to_value(&question.embedding).map_err(|e| {
            EvaluationDatasetRepositoryError::Internal(format!("serialize question embedding: {e}"))
        })?;

        sqlx::query(
            r#"
            INSERT INTO evaluation_questions (dataset_id, sequence, question, embedding)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (dataset_id, sequence) DO UPDATE SET
                question = $3,
                embedding = $4
            "#,
        )
        .bind(dataset_id)
        .bind(question.sequence as i32)
        .bind(&question.question)
        .bind(&embedding)
        .execute(&mut *tx)
        .await
        .map_err(|e| EvaluationDatasetRepositoryError::Internal(format!("save_question: {e}")))?;

        for (i, reference) in question.references.iter().enumerate() {
            let ref_embedding = serde_json::to_value(&reference.embedding).map_err(|e| {
                EvaluationDatasetRepositoryError::Internal(format!(
                    "serialize reference embedding: {e}"
                ))
            })?;

            sqlx::query(
                r#"
                INSERT INTO evaluation_references (
                    dataset_id, question_sequence, sequence, content, char_start, char_end, embedding
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (dataset_id, question_sequence, sequence) DO UPDATE SET
                    content = $4,
                    char_start = $5,
                    char_end = $6,
                    embedding = $7
                "#,
            )
            .bind(dataset_id)
            .bind(question.sequence as i32)
            .bind(i as i32)
            .bind(&reference.content)
            .bind(reference.char_start as i32)
            .bind(reference.char_end as i32)
            .bind(&ref_embedding)
            .execute(&mut *tx)
            .await
            .map_err(|e| EvaluationDatasetRepositoryError::Internal(format!("save_reference: {e}")))?;
        }

        tx.commit().await.map_err(|e| {
            EvaluationDatasetRepositoryError::Internal(format!("commit transaction: {e}"))
        })?;

        Ok(())
    }

    async fn list_for_document(
        &self,
        document_id: Uuid,
    ) -> Result<Vec<EvaluationDatasetReadModel>, EvaluationDatasetRepositoryError> {
        let rows: Vec<DatasetRow> = sqlx::query_as(
            r#"
            SELECT 
                dataset_id, document_id, document_version, content_hash, label,
                target_question_count, generation_model, generation_backend,
                excerpt_similarity_threshold_milli, duplicate_similarity_threshold_milli,
                embedding_model_id, status, question_count, rejection_count,
                failure_reason, created_at
            FROM evaluation_datasets 
            WHERE document_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            EvaluationDatasetRepositoryError::Internal(format!("list_for_document: {e}"))
        })?;

        Ok(rows
            .into_iter()
            .map(EvaluationDatasetReadModel::from)
            .collect())
    }

    async fn load_questions(
        &self,
        dataset_id: Uuid,
    ) -> Result<Vec<EvaluationQuestion>, EvaluationDatasetRepositoryError> {
        // This would involve joining with evaluation_questions and evaluation_references
        // For now, let's keep it simple and just load questions.
        // References can be loaded in a separate query or join.

        let question_rows: Vec<QuestionRow> = sqlx::query_as(
            "SELECT sequence, question, embedding FROM evaluation_questions WHERE dataset_id = $1 ORDER BY sequence ASC"
        )
        .bind(dataset_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| EvaluationDatasetRepositoryError::Internal(format!("load_questions: {e}")))?;

        let mut questions = Vec::new();
        for q_row in question_rows {
            let ref_rows: Vec<ReferenceRow> = sqlx::query_as(
                "SELECT content, char_start, char_end, embedding FROM evaluation_references WHERE dataset_id = $1 AND question_sequence = $2 ORDER BY sequence ASC"
            )
            .bind(dataset_id)
            .bind(q_row.sequence)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| EvaluationDatasetRepositoryError::Internal(format!("load_references: {e}")))?;

            questions.push(EvaluationQuestion {
                sequence: q_row.sequence as u32,
                question: q_row.question,
                references: ref_rows.into_iter().map(|r| r.into()).collect(),
                embedding: q_row.embedding.and_then(|v| serde_json::from_value(v).ok()),
            });
        }

        Ok(questions)
    }
}

#[derive(sqlx::FromRow)]
struct DatasetRow {
    dataset_id: Uuid,
    document_id: Uuid,
    document_version: i32,
    content_hash: String,
    label: String,
    target_question_count: i32,
    generation_model: String,
    generation_backend: String,
    excerpt_similarity_threshold_milli: i32,
    duplicate_similarity_threshold_milli: i32,
    embedding_model_id: Uuid,
    status: String,
    question_count: i32,
    rejection_count: i32,
    failure_reason: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<DatasetRow> for EvaluationDatasetReadModel {
    fn from(row: DatasetRow) -> Self {
        Self {
            dataset_id: row.dataset_id,
            document_id: row.document_id,
            document_version: row.document_version as u32,
            content_hash: row.content_hash,
            label: row.label,
            target_question_count: row.target_question_count as u32,
            generation_model: row.generation_model,
            generation_backend: row.generation_backend,
            excerpt_similarity_threshold_milli: row.excerpt_similarity_threshold_milli as u32,
            duplicate_similarity_threshold_milli: row.duplicate_similarity_threshold_milli as u32,
            embedding_model_id: row.embedding_model_id,
            status: serde_json::from_str(&row.status).unwrap_or(crate::server::domain::evaluation::dataset::aggregate::DatasetGenerationStatus::Generating),
            question_count: row.question_count as u32,
            rejection_count: row.rejection_count as u32,
            failure_reason: row.failure_reason,
            created_at: row.created_at.to_rfc3339(),
        }
    }
}

#[derive(sqlx::FromRow)]
struct QuestionRow {
    sequence: i32,
    question: String,
    embedding: Option<serde_json::Value>,
}

#[derive(sqlx::FromRow)]
struct ReferenceRow {
    content: String,
    char_start: i32,
    char_end: i32,
    embedding: Option<serde_json::Value>,
}

impl From<ReferenceRow> for crate::server::domain::evaluation::question::EvaluationReference {
    fn from(row: ReferenceRow) -> Self {
        Self {
            content: row.content,
            char_start: row.char_start as u32,
            char_end: row.char_end as u32,
            embedding: row.embedding.and_then(|v| serde_json::from_value(v).ok()),
        }
    }
}
