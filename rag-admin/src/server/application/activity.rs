//! In-memory projection of in-flight and recently-completed jobs.
//!
//! Drives the activity drawer in the UI. The registry is a side-table fed by
//! the same `EventBus` that powers cache invalidation, plus a small attach
//! API used by `SourceDocumentIngestService` to pin an SSE log feed onto its
//! activity row.
//!
//! The registry is process-local and ephemeral. Restarting the server clears
//! it — the UI tolerates that, because activity is best-effort presentation.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast::error::RecvError, Mutex};
use tracing::warn;
use uuid::Uuid;

use crate::server::event_sourcing::event_bus::EventBus;
use crate::shared::{classify, ActivityDelta, ActivityJobDto, ActivityStart, ActivityStatus};

/// How long terminal rows hang around after `finished_at` before the GC
/// removes them from the registry snapshot.
const TERMINAL_RETENTION: Duration = Duration::from_secs(15 * 60);

struct Row {
    dto: ActivityJobDto,
    /// Instant the row entered a terminal state; used by the GC to evict
    /// finished rows after `TERMINAL_RETENTION`.
    terminal_at: Option<Instant>,
}

struct State {
    rows: HashMap<Uuid, Row>,
    /// Stream URLs attached before their owning row materialised from an
    /// event. Applied at row-insert time. Drained eagerly when consumed.
    pending_streams: HashMap<Uuid, String>,
}

pub struct ActivityRegistry {
    state: Mutex<State>,
}

impl ActivityRegistry {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                rows: HashMap::new(),
                pending_streams: HashMap::new(),
            }),
        }
    }

    /// Snapshot the current registry, evicting terminal rows older than
    /// `TERMINAL_RETENTION` along the way.
    pub async fn snapshot(&self) -> Vec<ActivityJobDto> {
        let now = Instant::now();
        let mut state = self.state.lock().await;
        state.rows.retain(|_, row| match row.terminal_at {
            Some(at) => now.duration_since(at) < TERMINAL_RETENTION,
            None => true,
        });

        let mut out: Vec<ActivityJobDto> = state.rows.values().map(|r| r.dto.clone()).collect();
        out.sort_by(|a, b| match (a.status, b.status) {
            (ActivityStatus::Running, ActivityStatus::Running) => a.started_at.cmp(&b.started_at),
            (ActivityStatus::Running, _) => std::cmp::Ordering::Less,
            (_, ActivityStatus::Running) => std::cmp::Ordering::Greater,
            _ => b
                .finished_at
                .cmp(&a.finished_at)
                .then(b.started_at.cmp(&a.started_at)),
        });
        out
    }

    /// Attach an SSE log URL to an activity row. Used by ingest services that
    /// spawn a stage-level log feed in parallel with publishing
    /// `IngestRequested`. If the row doesn't exist yet (the event hasn't
    /// projected), the URL is stashed and applied when the row materialises.
    pub async fn attach_stream(&self, stream_id: Uuid, stream_url: String) {
        let mut state = self.state.lock().await;
        match state.rows.get_mut(&stream_id) {
            Some(row) => row.dto.stream_url = Some(stream_url),
            None => {
                state.pending_streams.insert(stream_id, stream_url);
            }
        }
    }

    async fn apply(&self, delta: ActivityDelta) {
        let mut state = self.state.lock().await;
        match delta {
            ActivityDelta::Start(ActivityStart {
                stream_id,
                aggregate_type,
                kind,
                label,
                started_at,
            }) => {
                let pending_stream = state.pending_streams.remove(&stream_id);
                let entry = state.rows.entry(stream_id).or_insert(Row {
                    dto: ActivityJobDto {
                        stream_id,
                        aggregate_type: aggregate_type.clone(),
                        kind,
                        label: label.clone(),
                        status: ActivityStatus::Running,
                        started_at: started_at.clone(),
                        finished_at: None,
                        stream_url: None,
                    },
                    terminal_at: None,
                });
                // If we'd previously seen a terminal event for the same stream
                // and a fresh Start arrives (e.g. a retry), reset to running.
                entry.dto.status = ActivityStatus::Running;
                entry.dto.started_at = started_at;
                entry.dto.finished_at = None;
                entry.terminal_at = None;
                entry.dto.aggregate_type = aggregate_type;
                entry.dto.kind = kind;
                entry.dto.label = label;
                if let Some(url) = pending_stream {
                    entry.dto.stream_url = Some(url);
                }
            }
            ActivityDelta::Complete {
                stream_id,
                occurred_at,
            } => {
                if let Some(row) = state.rows.get_mut(&stream_id) {
                    row.dto.status = ActivityStatus::Completed;
                    row.dto.finished_at = Some(occurred_at);
                    row.terminal_at = Some(Instant::now());
                }
            }
            ActivityDelta::Fail {
                stream_id,
                occurred_at,
            } => {
                if let Some(row) = state.rows.get_mut(&stream_id) {
                    row.dto.status = ActivityStatus::Failed;
                    row.dto.finished_at = Some(occurred_at);
                    row.terminal_at = Some(Instant::now());
                }
            }
            ActivityDelta::Remove { stream_id } => {
                state.rows.remove(&stream_id);
                state.pending_streams.remove(&stream_id);
            }
            ActivityDelta::Refresh { .. } => {}
        }
    }
}

impl Default for ActivityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn the background task that maintains the registry from the event bus.
/// Returns immediately; the task runs for the lifetime of the process.
pub fn spawn_activity_projection(registry: Arc<ActivityRegistry>, event_bus: Arc<EventBus>) {
    tokio::spawn(async move {
        let mut subscription = event_bus.subscribe();
        loop {
            match subscription.recv().await {
                Ok(event) => {
                    if let Some(delta) = classify(
                        event.stream_id,
                        &event.aggregate_type,
                        &event.event_type,
                        event.occurred_at.as_str(),
                    ) {
                        registry.apply(delta).await;
                    }
                }
                Err(RecvError::Lagged(n)) => {
                    warn!(dropped = n, "activity projection subscriber lagged");
                }
                Err(RecvError::Closed) => return,
            }
        }
    });
}
