use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::{aggregate_type, PublishedEvent};

/// Build an `ActivityDelta` from the fields every event carries on both
/// the server (`server::event_sourcing::envelope::PublishedEvent`) and the
/// client (`shared::PublishedEvent`). The two have identical wire shapes but
/// different Rust types — taking primitives lets both call sites share the
/// classifier without a conversion hop.
pub fn classify(
    stream_id: Uuid,
    aggregate_type_str: &str,
    event_type: &str,
    occurred_at: &str,
    event_data: &serde_json::Value,
) -> Option<ActivityDelta> {
    match (aggregate_type_str, event_type) {
        (aggregate_type::INDEXING, "IngestRequested") => {
            Some(ActivityDelta::Start(ActivityStart {
                stream_id,
                aggregate_type: aggregate_type_str.to_string(),
                kind: ActivityKind::Indexing,
                label: format!("Indexing {}", short_id(stream_id)),
                started_at: occurred_at.to_string(),
            }))
        }
        (aggregate_type::INDEXING, "IndexingCompleted") => Some(ActivityDelta::Complete {
            stream_id,
            occurred_at: occurred_at.to_string(),
        }),
        (aggregate_type::INDEXING, "IngestionFailed") => Some(ActivityDelta::Fail {
            stream_id,
            occurred_at: occurred_at.to_string(),
        }),
        (aggregate_type::INDEXING, "IndexingRemoved") => Some(ActivityDelta::Remove { stream_id }),
        (aggregate_type::INDEXING, "ChunkingCompleted" | "EmbeddingCompleted") => {
            let auto_advance = event_data
                .get("auto_advance")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            if auto_advance {
                Some(ActivityDelta::Refresh { stream_id })
            } else {
                Some(ActivityDelta::Complete {
                    stream_id,
                    occurred_at: occurred_at.to_string(),
                })
            }
        }
        (
            aggregate_type::INDEXING,
            "ChunkingRequeued" | "EmbeddingRequeued" | "IndexingRequeued",
        ) => Some(ActivityDelta::Start(ActivityStart {
            stream_id,
            aggregate_type: aggregate_type_str.to_string(),
            kind: ActivityKind::Indexing,
            label: format!("Indexing {}", short_id(stream_id)),
            started_at: occurred_at.to_string(),
        })),
        (aggregate_type::INDEXING, "IngestionRetried") => {
            Some(ActivityDelta::Refresh { stream_id })
        }

        (aggregate_type::EVALUATION_DATASET, "DatasetGenerationRequested") => {
            Some(ActivityDelta::Start(ActivityStart {
                stream_id,
                aggregate_type: aggregate_type_str.to_string(),
                kind: ActivityKind::EvaluationDataset,
                label: format!("Dataset {}", short_id(stream_id)),
                started_at: occurred_at.to_string(),
            }))
        }
        (aggregate_type::EVALUATION_DATASET, "DatasetGenerationCompleted") => {
            Some(ActivityDelta::Complete {
                stream_id,
                occurred_at: occurred_at.to_string(),
            })
        }
        (aggregate_type::EVALUATION_DATASET, "DatasetGenerationFailed") => {
            Some(ActivityDelta::Fail {
                stream_id,
                occurred_at: occurred_at.to_string(),
            })
        }

        (aggregate_type::EVALUATION_RUN, "RunRequested") => {
            Some(ActivityDelta::Start(ActivityStart {
                stream_id,
                aggregate_type: aggregate_type_str.to_string(),
                kind: ActivityKind::EvaluationRun,
                label: format!("Run {}", short_id(stream_id)),
                started_at: occurred_at.to_string(),
            }))
        }
        (aggregate_type::EVALUATION_RUN, "RunCompleted") => Some(ActivityDelta::Complete {
            stream_id,
            occurred_at: occurred_at.to_string(),
        }),
        (aggregate_type::EVALUATION_RUN, "RunFailed") => Some(ActivityDelta::Fail {
            stream_id,
            occurred_at: occurred_at.to_string(),
        }),

        _ => None,
    }
}

/// Wire-format row backing one entry in the activity drawer.
///
/// The same shape is used by `list_active_jobs` (server) and the client's
/// `ActivityState` projection (hydrated from the event bus). Rows are keyed
/// by `stream_id` (the aggregate id) so a single activity row is a function
/// of one aggregate's lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityJobDto {
    pub stream_id: Uuid,
    pub aggregate_type: String,
    pub kind: ActivityKind,
    pub label: String,
    pub status: ActivityStatus,
    pub started_at: String,
    pub finished_at: Option<String>,
    /// SSE endpoint for the job's stdout-style log feed, when one exists.
    /// Indexing rows attach this from `SourceDocumentIngestService`; eval and
    /// dataset rows leave it `None` and the drawer falls back to rendering
    /// the aggregate's event sequence.
    pub stream_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityKind {
    Indexing,
    EvaluationDataset,
    EvaluationRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityStatus {
    Running,
    Completed,
    Failed,
}

impl ActivityStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, ActivityStatus::Completed | ActivityStatus::Failed)
    }
}

/// What an observed event implies for the activity registry. Used identically
/// on the server (to build the authoritative projection) and the client (to
/// keep the local snapshot in sync without round-tripping every event).
#[derive(Debug, Clone)]
pub enum ActivityDelta {
    Start(ActivityStart),
    Complete {
        stream_id: Uuid,
        occurred_at: String,
    },
    Fail {
        stream_id: Uuid,
        occurred_at: String,
    },
    Remove {
        stream_id: Uuid,
    },
    Refresh {
        stream_id: Uuid,
    },
}

#[derive(Debug, Clone)]
pub struct ActivityStart {
    pub stream_id: Uuid,
    pub aggregate_type: String,
    pub kind: ActivityKind,
    pub label: String,
    pub started_at: String,
}

/// Map a `shared::PublishedEvent` (used by the client) to its delta.
pub fn classify_event(event: &PublishedEvent) -> Option<ActivityDelta> {
    classify(
        event.stream_id,
        &event.aggregate_type,
        &event.event_type,
        &event.occurred_at,
        &event.event_data,
    )
}

fn short_id(id: Uuid) -> String {
    id.to_string()[..8].to_string()
}
