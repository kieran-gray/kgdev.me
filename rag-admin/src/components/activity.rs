//! Client-side mirror of the server `ActivityRegistry`.
//!
//! On mount we fetch a snapshot from `list_active_jobs` so the drawer is
//! populated even before the websocket sends any events. After mount we
//! apply event-bus deltas locally — the same `classify` function the server
//! projection uses, so the two stay in sync without round-trips.
//!
//! The drawer's open/closed state is mirrored to the URL via
//! `?activity=open` so a page reload re-opens the drawer.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_location, use_navigate};
use leptos_router::NavigateOptions;

use crate::components::event_bus::use_event_bus;
use crate::server_functions::jobs::list_active_jobs;
use crate::shared::{
    classify_event, ActivityDelta, ActivityJobDto, ActivityKind, ActivityStart, ActivityStatus,
};

/// Activity context. `Copy`-able handle to a small set of signals.
///
/// The write half (`set_rows`) is intentionally kept private inside
/// `provide_activity_state` — consumers should never mutate the list; updates
/// arrive via the event bus and the `list_active_jobs` snapshot.
#[derive(Clone, Copy)]
pub struct ActivityState {
    pub rows: ReadSignal<Vec<ActivityJobDto>>,
    pub open: Memo<bool>,
}

impl ActivityState {
    /// Authoritative count of running jobs. Used by the nav activity dot.
    pub fn running_count(&self) -> usize {
        self.rows.with(|rows| {
            rows.iter()
                .filter(|r| r.status == ActivityStatus::Running)
                .count()
        })
    }
}

/// Initialise the activity state and put it in context. Call once from `App`,
/// after `provide_event_bus`.
pub fn provide_activity_state() {
    let (rows, set_rows) = signal::<Vec<ActivityJobDto>>(Vec::new());

    // ── Reflect ?activity=open into a memo, navigation back into the URL ──
    let location = use_location();
    let open = Memo::new(move |_| {
        location
            .query
            .with(|q| q.get("activity").map(|v| v == "open").unwrap_or(false))
    });

    let state = ActivityState { rows, open };
    provide_context(state);

    // ── Seed from server snapshot on mount ────────────────────────────────
    Effect::new(move |prev: Option<()>| {
        if prev.is_none() {
            spawn_local(async move {
                match list_active_jobs().await {
                    Ok(snapshot) => set_rows.set(snapshot),
                    Err(err) => {
                        leptos::logging::warn!("list_active_jobs failed: {err}");
                    }
                }
            });
        }
    });

    let bus = use_event_bus();
    Effect::new(move |_| {
        if let Some(event) = bus.last_event.get() {
            if let Some(delta) = classify_event(&event) {
                let needs_url_refresh = matches!(
                    &delta,
                    ActivityDelta::Start(ActivityStart {
                        kind: ActivityKind::Indexing,
                        ..
                    }) | ActivityDelta::Refresh { .. }
                );
                set_rows.update(|rows| apply_delta(rows, delta));
                if needs_url_refresh {
                    spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(150).await;
                        if let Ok(snapshot) = list_active_jobs().await {
                            set_rows.set(snapshot);
                        }
                    });
                }
            }
        }
    });

    // ── On reconnect, re-seed from the server to recover from missed events
    Effect::new(move |prev_epoch: Option<u32>| {
        let epoch = bus.epoch.get();
        if prev_epoch.is_some() && prev_epoch != Some(epoch) {
            spawn_local(async move {
                if let Ok(snapshot) = list_active_jobs().await {
                    set_rows.set(snapshot);
                }
            });
        }
        epoch
    });
}

/// Read the activity state from context.
pub fn use_activity_state() -> ActivityState {
    use_context::<ActivityState>().expect("ActivityState context must be provided in App")
}

/// Toggle the drawer by mutating the `activity` query param. Navigating
/// (rather than touching local state directly) keeps the URL persistent
/// across reloads.
pub fn toggle_drawer(open: bool) {
    let navigate = use_navigate();
    let location = use_location();
    let path = location.pathname.get_untracked();
    let mut params = location.query.get_untracked();

    if open {
        params.insert("activity", "open".to_string());
    } else {
        params.remove("activity");
    }

    let qs = params.to_query_string();
    let url = if qs.is_empty() {
        path
    } else {
        format!("{path}{qs}")
    };
    navigate(&url, NavigateOptions::default());
}

fn apply_delta(rows: &mut Vec<ActivityJobDto>, delta: ActivityDelta) {
    match delta {
        ActivityDelta::Start(ActivityStart {
            stream_id,
            aggregate_type,
            kind,
            label,
            started_at,
        }) => match rows.iter_mut().find(|r| r.stream_id == stream_id) {
            Some(row) => {
                row.status = ActivityStatus::Running;
                row.started_at = started_at;
                row.finished_at = None;
                row.label = label;
                row.aggregate_type = aggregate_type;
                row.kind = kind;
            }
            None => rows.push(ActivityJobDto {
                stream_id,
                aggregate_type,
                kind,
                label,
                status: ActivityStatus::Running,
                started_at,
                finished_at: None,
                stream_url: None,
            }),
        },
        ActivityDelta::Complete {
            stream_id,
            occurred_at,
        } => {
            if let Some(row) = rows.iter_mut().find(|r| r.stream_id == stream_id) {
                row.status = ActivityStatus::Completed;
                row.finished_at = Some(occurred_at);
            }
        }
        ActivityDelta::Fail {
            stream_id,
            occurred_at,
        } => {
            if let Some(row) = rows.iter_mut().find(|r| r.stream_id == stream_id) {
                row.status = ActivityStatus::Failed;
                row.finished_at = Some(occurred_at);
            }
        }
        ActivityDelta::Remove { stream_id } => {
            rows.retain(|r| r.stream_id != stream_id);
        }
        ActivityDelta::Refresh { .. } => {}
    }
}
