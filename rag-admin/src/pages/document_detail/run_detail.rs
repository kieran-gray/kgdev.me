use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;

use crate::components::event_bus::use_invalidator;
use crate::components::primitives::{
    EmptyState, MetricBar, MetricKind, PageHeader, Status, StatusPill, Surface,
};
use crate::server_functions::evaluation::get_run;
use crate::shared::{
    evaluation_score, EvaluationMetrics, EvaluationRunDto, EvaluationVariantResult,
};

/// `/runs/:run_id` — deep-dive into a single evaluation run.
///
/// Routes outside the document subtree so a future Evaluations leaderboard can
/// link here without round-tripping through the document detail page.
#[component]
pub fn RunDetailPage() -> impl IntoView {
    let params = use_params_map();
    let run_id = Memo::new(move |_| {
        params
            .with(|p| p.get("run_id").unwrap_or_default().to_string())
            .parse::<Uuid>()
            .ok()
    });

    let run_invalidator = use_invalidator(|e| e.from_any(&["EvaluationRun"]));
    let run = Resource::new(
        move || (run_id.get(), run_invalidator.get()),
        move |(id, _)| async move {
            match id {
                Some(id) => get_run(id).await.map_err(|e| e.to_string()),
                None => Ok(None),
            }
        },
    );

    view! {
        <Transition fallback=|| view! { <p class="muted">"Loading run…"</p> }>
            {move || run.get().map(|res| match res {
                Err(e) => view! {
                    <Surface><div class="log-line-error">{format!("Failed to load: {e}")}</div></Surface>
                }.into_any(),
                Ok(None) => view! {
                    <Surface>
                        <EmptyState
                            title="Run not found"
                            body="This run id is unknown or has been removed.".to_string()
                        />
                    </Surface>
                }.into_any(),
                Ok(Some(r)) => view! { <RunView run=r /> }.into_any(),
            })}
        </Transition>
    }
}

#[component]
fn RunView(run: EvaluationRunDto) -> impl IntoView {
    let (status_kind, status_label) = match run.status.as_str() {
        "completed" => (Status::Ok, "Completed"),
        "failed" => (Status::Fail, "Failed"),
        "running" => (Status::Pending, "Running"),
        _ => (Status::Neutral, "Unknown"),
    };
    let short = run.run_id.to_string()[..8].to_string();

    // Sort variants by overall score, descending. The first variant (highest
    // score) gets highlighted as the leader.
    let mut variants = run.variants;
    variants.sort_by(|a, b| {
        evaluation_score(&b.metrics)
            .partial_cmp(&evaluation_score(&a.metrics))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    view! {
        <div>
            <PageHeader
                title=format!("run-{short}")
                eyebrow="Evaluations / Run".to_string()
                subtitle=format!("{} variants · {}", variants.len(), run.created_at)
                actions=Box::new(move || view! {
                    <StatusPill label=status_label.to_string() kind=status_kind />
                }.into_any())
            />

            <div class="mb-4">
                <A href="/evaluations" attr:class="muted text-sm">"← Back to evaluations"</A>
            </div>

            {if variants.is_empty() {
                view! {
                    <Surface>
                        <EmptyState
                            title="No variants yet"
                            body="The run may still be in progress; variants land here as they're scored.".to_string()
                        />
                    </Surface>
                }.into_any()
            } else {
                view! {
                    <div class="space-y-4">
                        {variants.into_iter().enumerate().map(|(i, v)| view! {
                            <VariantCard variant=v leader=i == 0 />
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[component]
fn VariantCard(variant: EvaluationVariantResult, leader: bool) -> impl IntoView {
    let kind = if leader { MetricKind::Best } else { MetricKind::Default };
    let score = evaluation_score(&variant.metrics);
    let label = variant.variant.label.clone();
    let split = variant.split.as_str().to_string();
    let chunk_count = variant.chunk_count;
    let avg_tokens = variant.average_chunk_tokens;
    let selected = variant.selected;

    view! {
        <div class=format!(
            "surface-raised rounded p-4 {}",
            if leader { "border-l-2 border-l-[var(--color-accent)]" } else { "" }
        )>
            <div class="flex items-center justify-between mb-3 gap-3">
                <div class="flex items-center gap-3">
                    {leader.then(|| view! { <span class="text-[var(--color-accent)]">"★"</span> })}
                    <span class="font-mono text-base">{label}</span>
                    <span class="text-xs muted">{format!("{chunk_count} chunks · avg {avg_tokens} tokens")}</span>
                </div>
                <div class="flex items-center gap-3">
                    <span class="text-xs muted">{format!("split: {split}")}</span>
                    {selected.then(|| view! {
                        <span class="pill pill-ok">"selected"</span>
                    })}
                    <span class="font-mono text-sm">{format!("score {:.2}", score)}</span>
                </div>
            </div>
            <MetricBars metrics=variant.metrics kind=kind />
        </div>
    }
}

#[component]
fn MetricBars(metrics: EvaluationMetrics, kind: MetricKind) -> impl IntoView {
    view! {
        <div class="space-y-1.5">
            <MetricBar label="R".to_string() value=metrics.recall_mean kind=kind stddev=metrics.recall_std />
            <MetricBar label="P".to_string() value=metrics.precision_mean kind=kind stddev=metrics.precision_std />
            <MetricBar label="IoU".to_string() value=metrics.iou_mean kind=kind stddev=metrics.iou_std />
            <MetricBar label="Pω".to_string() value=metrics.precision_omega_mean kind=kind stddev=metrics.precision_omega_std />
        </div>
    }
}
