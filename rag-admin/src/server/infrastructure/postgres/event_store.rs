use std::marker::PhantomData;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::server::application::configuration::ports::ConfigurationEventStore;
use crate::server::application::indexing::ports::IndexingEventStore;
use crate::server::application::source_document::ports::SourceDocumentEventStore;
use crate::server::application::AppError;
use crate::server::domain::configuration::events::ConfigurationEvent;
use crate::server::domain::indexing::events::IndexingEvent;
use crate::server::domain::source_document::events::SourceDocumentEvent;

pub struct PostgresEventStore<E> {
    pool: PgPool,
    aggregate_type: &'static str,
    _phantom: PhantomData<E>,
}

impl<E> PostgresEventStore<E> {
    pub fn new(pool: PgPool, aggregate_type: &'static str) -> Self {
        Self {
            pool,
            aggregate_type,
            _phantom: PhantomData,
        }
    }
}

impl<E> PostgresEventStore<E>
where
    E: Serialize + DeserializeOwned + Send + Sync,
{
    pub async fn load_events(&self, aggregate_id: Uuid) -> Result<Vec<E>, AppError> {
        let rows: Vec<(serde_json::Value,)> = sqlx::query_as(
            "SELECT event_data FROM events WHERE stream_id = $1 ORDER BY position ASC",
        )
        .bind(aggregate_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("load events: {e}")))?;

        rows.into_iter()
            .map(|(json,)| {
                serde_json::from_value(json)
                    .map_err(|e| AppError::Internal(format!("deserialize event: {e}")))
            })
            .collect()
    }

    pub async fn append_events(
        &self,
        aggregate_id: Uuid,
        expected_version: usize,
        events: &[E],
    ) -> Result<(), AppError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(format!("begin transaction: {e}")))?;

        let (current_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM events WHERE stream_id = $1")
                .bind(aggregate_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| AppError::Internal(format!("read stream version: {e}")))?;

        if current_count as usize != expected_version {
            return Err(AppError::Validation(format!(
                "{} stream version conflict: expected {expected_version}, actual {current_count}",
                self.aggregate_type
            )));
        }

        for (i, event) in events.iter().enumerate() {
            let position = (expected_version + i) as i64;
            let json = serde_json::to_value(event)
                .map_err(|e| AppError::Internal(format!("serialize event: {e}")))?;
            let event_type = json
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            sqlx::query(
                "INSERT INTO events (stream_id, aggregate_type, position, event_type, event_data) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(aggregate_id)
            .bind(self.aggregate_type)
            .bind(position)
            .bind(&event_type)
            .bind(&json)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(ref db_err) = e {
                    if db_err.constraint() == Some("events_stream_position_unique") {
                        return AppError::Validation(format!(
                            "{} stream version conflict at position {position}",
                            self.aggregate_type
                        ));
                    }
                }
                AppError::Internal(format!("append event: {e}"))
            })?;
        }

        tx.commit()
            .await
            .map_err(|e| AppError::Internal(format!("commit transaction: {e}")))?;

        Ok(())
    }
}

#[async_trait]
impl ConfigurationEventStore for PostgresEventStore<ConfigurationEvent> {
    async fn load(&self, aggregate_id: Uuid) -> Result<Vec<ConfigurationEvent>, AppError> {
        self.load_events(aggregate_id).await
    }

    async fn append(
        &self,
        aggregate_id: Uuid,
        expected_version: usize,
        events: &[ConfigurationEvent],
    ) -> Result<(), AppError> {
        self.append_events(aggregate_id, expected_version, events)
            .await
    }
}

#[async_trait]
impl SourceDocumentEventStore for PostgresEventStore<SourceDocumentEvent> {
    async fn load(&self, aggregate_id: Uuid) -> Result<Vec<SourceDocumentEvent>, AppError> {
        self.load_events(aggregate_id).await
    }

    async fn append(
        &self,
        aggregate_id: Uuid,
        expected_version: usize,
        events: &[SourceDocumentEvent],
    ) -> Result<(), AppError> {
        self.append_events(aggregate_id, expected_version, events)
            .await
    }
}

#[async_trait]
impl IndexingEventStore for PostgresEventStore<IndexingEvent> {
    async fn load(&self, aggregate_id: Uuid) -> Result<Vec<IndexingEvent>, AppError> {
        self.load_events(aggregate_id).await
    }

    async fn append(
        &self,
        aggregate_id: Uuid,
        expected_version: usize,
        events: &[IndexingEvent],
    ) -> Result<(), AppError> {
        self.append_events(aggregate_id, expected_version, events)
            .await
    }
}
