use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_location;

use crate::components::event_bus::{use_event_bus, ConnectionState};
use crate::shared::PublishedEvent;

/// Top navigation bar. One row, six destinations, an activity dot on the right.
///
/// The activity dot mirrors the event-bus connection state plus a running
/// count of in-flight aggregate jobs (best-effort, derived from recently
/// observed start/finish events).
#[component]
pub fn AppNav() -> impl IntoView {
    let location = use_location();
    let bus = use_event_bus();

    // Static destination list — kept in one place so a fifth screen doesn't
    // require touching multiple files.
    let destinations: &[(&'static str, &'static str)] = &[
        ("/", "Documents"),
        ("/evaluations", "Evaluations"),
        ("/pipelines", "Pipelines"),
        ("/chunking", "Chunking"),
        ("/playground", "Playground"),
    ];

    let is_active = move |href: &'static str| {
        let path = location.pathname.get();
        if href == "/" {
            path == "/" || path.starts_with("/documents")
        } else {
            path == href || path.starts_with(&format!("{href}/"))
        }
    };

    // Best-effort in-flight job counter. Updates from the event bus: increment
    // on a "started" event, decrement on a "completed" / "failed" event. Per-
    // event semantics are encoded in `event_indicates_*` below so adding new
    // aggregate types is a one-line change.
    let (active_jobs, set_active_jobs) = signal(0u32);

    Effect::new(move |_| {
        if let Some(event) = bus.last_event.get() {
            apply_event_to_counter(&event, set_active_jobs);
        }
    });

    Effect::new(move |prev: Option<u32>| {
        // On reconnect, reset the counter — we don't trust our cached count
        // after a gap in the event stream.
        let epoch = bus.epoch.get();
        if prev.is_some() && prev != Some(epoch) {
            set_active_jobs.set(0);
        }
        epoch
    });

    let activity_dot_class = move || {
        let active = active_jobs.get() > 0;
        let connected = matches!(bus.connection.get(), ConnectionState::Open);
        match (connected, active) {
            (false, _) => "activity-dot activity-dot-fail",
            (true, false) => "activity-dot",
            (true, true) => "activity-dot activity-dot-active",
        }
    };

    let activity_title = move || match (bus.connection.get(), active_jobs.get()) {
        (ConnectionState::Open, 0) => "Connected · idle".to_string(),
        (ConnectionState::Open, n) => format!("Connected · {n} active job(s)"),
        (ConnectionState::Connecting, _) => "Connecting…".to_string(),
        (ConnectionState::Closed, _) => "Disconnected".to_string(),
    };

    view! {
        <header class="app-header">
            <div class="max-w-6xl mx-auto px-6 py-3 flex items-center gap-8">
                <A href="/" attr:class="wordmark" attr:aria-label="rag-admin home">
                    "rag-admin"
                </A>
                <nav class="flex gap-5 text-sm">
                    {destinations.iter().copied().map(|(href, label)| {
                        let active = move || is_active(href);
                        view! {
                            <A
                                href=href
                                attr:class="app-nav-link"
                                attr:aria-current=move || if active() { "page" } else { "" }
                            >
                                {label}
                            </A>
                        }
                    }).collect_view()}
                </nav>
                <div class="toolbar-spacer"></div>
                <button
                    type="button"
                    class="btn-ghost btn flex items-center gap-2"
                    title=activity_title
                    aria-label="Activity"
                >
                    <span class=activity_dot_class></span>
                    <span class="muted text-xs">
                        {move || active_jobs.get().to_string()}
                    </span>
                </button>
                <A href="/settings" attr:class="btn-ghost btn" attr:title="Settings">
                    "Settings"
                </A>
            </div>
        </header>
    }
}

/// Update the in-flight job counter based on an observed event.
///
/// Started: increments. Completed/failed/removed: decrements (saturating).
/// Unknown events are ignored — the activity counter is best-effort UX, not
/// authoritative state.
fn apply_event_to_counter(event: &PublishedEvent, set_count: WriteSignal<u32>) {
    let delta: i32 = match (event.aggregate_type.as_str(), event.event_type.as_str()) {
        ("Indexing", "IngestRequested") => 1,
        ("Indexing", "IndexingCompleted" | "IngestionFailed" | "IndexingRemoved") => -1,
        ("EvaluationRun", "RunRequested") => 1,
        ("EvaluationRun", "RunCompleted" | "RunFailed") => -1,
        ("EvaluationDataset", "DatasetRequested") => 1,
        ("EvaluationDataset", "Completed" | "Failed") => -1,
        _ => 0,
    };

    if delta == 0 {
        return;
    }

    set_count.update(|c| {
        if delta > 0 {
            *c = c.saturating_add(delta as u32);
        } else {
            *c = c.saturating_sub((-delta) as u32);
        }
    });
}
