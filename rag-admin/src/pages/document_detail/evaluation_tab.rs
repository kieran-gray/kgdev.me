use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

use crate::components::log_panel::LogPanel;
use crate::server_functions::evaluation::{
    get_datasets_for_document, get_run, get_runs_for_document, start_generate_synthetic_dataset,
    start_run_evaluation,
};
use crate::shared::{
    ChunkingConfig, ChunkingVariant, EvaluationDatasetSummaryDto, EvaluationRunOptions,
    EvaluationRunSummaryDto, EvaluationVariantResult, LogEvent, PipelineConfigurationDto,
    RunEvaluationRequestDto, SectionChunkingConfig,
};

use super::utils::open_event_stream;

#[component]
pub fn EvaluationTab(
    document_id: Uuid,
    pipelines: Vec<PipelineConfigurationDto>,
) -> impl IntoView {
    let pipelines_stored = StoredValue::new(pipelines);

    let (active_dataset, set_active_dataset) = signal::<Option<Uuid>>(None);
    let (active_pipeline, set_active_pipeline) = signal::<Option<Uuid>>(None);
    let (selected_run, set_selected_run) = signal::<Option<Uuid>>(None);
    let (log_events, set_log_events) = signal::<Vec<LogEvent>>(Vec::new());
    let (job_running, set_job_running) = signal(false);
    let (dataset_label, set_dataset_label) = signal("synthetic-default".to_string());
    let (variant_tokens, set_variant_tokens) = signal(512u32);

    let (datasets_version, set_datasets_version) = signal(0u32);
    let (runs_version, set_runs_version) = signal(0u32);

    // Refresh datasets and runs when a job finishes.
    Effect::watch(
        move || job_running.get(),
        move |running, prev, _| {
            if let Some(true) = prev {
                if !running {
                    set_datasets_version.update(|v| *v += 1);
                    set_runs_version.update(|v| *v += 1);
                }
            }
        },
        false,
    );

    let datasets = Resource::new(
        move || datasets_version.get(),
        move |_| async move {
            get_datasets_for_document(document_id)
                .await
                .unwrap_or_default()
        },
    );

    let runs = Resource::new(
        move || runs_version.get(),
        move |_| async move {
            get_runs_for_document(document_id)
                .await
                .unwrap_or_default()
        },
    );

    let run_detail = Resource::new(
        move || selected_run.get(),
        move |run_id| async move {
            match run_id {
                Some(id) => get_run(id).await.unwrap_or(None),
                None => None,
            }
        },
    );

    let can_run = move || active_dataset.get().is_some() && active_pipeline.get().is_some();

    view! {
        <div class="space-y-6 animate-in fade-in duration-200">

            // ── Top row: Pipeline selector (left) + Dataset panel (right) ─────────
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">

                // ── Pipeline selector ────────────────────────────────────────────
                <div class="space-y-2">
                    <span class="tech-label opacity-40">"PIPELINE_SELECTOR"</span>
                    <div class="card-outer p-4 space-y-2">
                        {move || {
                            let ps = pipelines_stored.get_value();
                            if ps.is_empty() {
                                return view! {
                                    <p class="tech-label opacity-30 text-xs">
                                        "No pipelines — add one in PIPELINE_CONFIG"
                                    </p>
                                }.into_any();
                            }
                            ps.into_iter().map(|pc| {
                                let pc_id = pc.pipeline_configuration_id;
                                let is_active = move || active_pipeline.get() == Some(pc_id);
                                view! {
                                    <button
                                        class=move || format!(
                                            "w-full text-left px-3 py-2 rounded border text-xs font-mono transition-colors {}",
                                            if is_active() {
                                                "border-[var(--color-accent)] bg-[var(--color-accent)]/10 text-[var(--color-accent)]"
                                            } else {
                                                "border-[var(--color-border)] hover:border-[var(--color-accent)]/50"
                                            }
                                        )
                                        on:click=move |_| set_active_pipeline.set(Some(pc_id))
                                    >
                                        <span class="opacity-40 mr-1">"▶"</span>
                                        {pc.name}
                                    </button>
                                }
                            }).collect_view().into_any()
                        }}
                    </div>
                </div>

                // ── Dataset panel ────────────────────────────────────────────────
                <div class="space-y-2">
                    <span class="tech-label opacity-40">"DATASET_PANEL"</span>
                    <div class="card-outer p-4 space-y-3">

                        // Generate form
                        <div class="space-y-1">
                            <span class="tech-label opacity-50 text-[10px]">"GENERATE_NEW_DATASET"</span>
                            <div class="flex gap-2 mt-1">
                                <input
                                    class="flex-1 px-2 py-1 text-xs font-mono bg-[var(--color-card-inner)] border border-[var(--color-border)] rounded focus:outline-none focus:border-[var(--color-accent)]"
                                    placeholder="label (e.g. synthetic-default)"
                                    prop:value=move || dataset_label.get()
                                    on:input=move |ev| set_dataset_label.set(event_target_value(&ev))
                                />
                                <button
                                    class="px-3 py-1 text-[10px] font-bold tracking-widest border border-[var(--color-accent)] text-[var(--color-accent)] hover:bg-[var(--color-accent)]/10 transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
                                    disabled=move || job_running.get()
                                    on:click=move |_| {
                                        let label = dataset_label.get();
                                        set_log_events.set(vec![]);
                                        set_job_running.set(true);
                                        spawn_local(async move {
                                            match start_generate_synthetic_dataset(document_id, label).await {
                                                Ok(job) => open_event_stream(job.stream_url, set_log_events, set_job_running),
                                                Err(e) => {
                                                    set_job_running.set(false);
                                                    set_log_events.update(|evs| evs.push(LogEvent {
                                                        level: crate::shared::LogLevel::Error,
                                                        message: format!("{e}"),
                                                    }));
                                                }
                                            }
                                        });
                                    }
                                >
                                    {move || if job_running.get() { "GENERATING..." } else { "GENERATE" }}
                                </button>
                            </div>
                        </div>

                        // Dataset list
                        <Transition fallback=|| view! {
                            <p class="tech-label animate-pulse text-[10px]">"LOADING..."</p>
                        }>
                            {move || {
                                let ds = datasets.get().unwrap_or_default();
                                if ds.is_empty() {
                                    return view! {
                                        <p class="tech-label opacity-30 text-[10px]">
                                            "No datasets yet — generate one above."
                                        </p>
                                    }.into_any();
                                }
                                view! {
                                    <div class="space-y-1">
                                        <span class="tech-label opacity-40 text-[10px]">"EXISTING_DATASETS"</span>
                                        {ds.into_iter().map(|d| {
                                            let did = d.dataset_id;
                                            let is_active = move || active_dataset.get() == Some(did);
                                            view! {
                                                <DatasetRow
                                                    dataset=d
                                                    is_active=is_active
                                                    on_select=move || set_active_dataset.set(Some(did))
                                                />
                                            }
                                        }).collect_view()}
                                    </div>
                                }.into_any()
                            }}
                        </Transition>
                    </div>
                </div>
            </div>

            // ── Run launcher ──────────────────────────────────────────────────────
            <div class="space-y-2">
                <span class="tech-label opacity-40">"RUN_LAUNCHER"</span>
                <div class="card-outer p-4 space-y-3">
                    <div class="space-y-1">
                        <span class="tech-label opacity-50 text-[10px]">"CHUNKING_VARIANT"</span>
                        <div class="flex gap-2 flex-wrap mt-1">
                            {[256u32, 384, 512, 768, 1024].into_iter().map(|t| {
                                let is_active = move || variant_tokens.get() == t;
                                view! {
                                    <button
                                        class=move || format!(
                                            "px-2 py-1 text-[10px] font-mono tracking-widest border rounded transition-colors {}",
                                            if is_active() {
                                                "border-[var(--color-accent)] text-[var(--color-accent)]"
                                            } else {
                                                "border-[var(--color-border)] opacity-50 hover:opacity-100"
                                            }
                                        )
                                        on:click=move |_| set_variant_tokens.set(t)
                                    >
                                        {format!("section:{t}")}
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                    </div>

                    <button
                        class="w-full py-2 text-xs font-bold tracking-widest border border-[var(--color-accent)] text-[var(--color-accent)] hover:bg-[var(--color-accent)]/10 transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
                        disabled=move || !can_run() || job_running.get()
                        on:click=move |_| {
                            let Some(dataset_id) = active_dataset.get() else { return; };
                            let Some(pipeline_id) = active_pipeline.get() else { return; };
                            let tokens = variant_tokens.get();
                            let request = RunEvaluationRequestDto {
                                dataset_id,
                                pipeline_configuration_id: pipeline_id,
                                variants: vec![ChunkingVariant {
                                    label: format!("section:{tokens}"),
                                    config: ChunkingConfig::Section(SectionChunkingConfig {
                                        max_section_tokens: tokens,
                                    }),
                                }],
                                options: vec![EvaluationRunOptions::default()],
                            };
                            set_log_events.set(vec![]);
                            set_job_running.set(true);
                            spawn_local(async move {
                                match start_run_evaluation(request).await {
                                    Ok(job) => open_event_stream(job.stream_url, set_log_events, set_job_running),
                                    Err(e) => {
                                        set_job_running.set(false);
                                        set_log_events.update(|evs| evs.push(LogEvent {
                                            level: crate::shared::LogLevel::Error,
                                            message: format!("{e}"),
                                        }));
                                    }
                                }
                            });
                        }
                    >
                        {move || {
                            if job_running.get() {
                                "RUNNING..."
                            } else if !can_run() {
                                "SELECT_DATASET_AND_PIPELINE"
                            } else {
                                "START_EVALUATION_RUN"
                            }
                        }}
                    </button>
                </div>
            </div>

            // ── Job log (visible only while active) ───────────────────────────────
            {move || {
                if log_events.with(|evs| evs.is_empty()) && !job_running.get() {
                    return view! { <div></div> }.into_any();
                }
                view! {
                    <div class="space-y-2">
                        <span class="tech-label opacity-40">"JOB_LOG"</span>
                        <div class="h-48 overflow-y-auto card-outer p-3">
                            <LogPanel events=log_events />
                        </div>
                    </div>
                }.into_any()
            }}

            // ── Run history ───────────────────────────────────────────────────────
            <div class="space-y-2">
                <span class="tech-label opacity-40">"RUN_HISTORY"</span>
                <Transition fallback=|| view! {
                    <p class="tech-label animate-pulse text-[10px]">"LOADING_RUNS..."</p>
                }>
                    {move || {
                        let rs = runs.get().unwrap_or_default();
                        if rs.is_empty() {
                            return view! {
                                <div class="card-outer p-8 flex items-center justify-center border-dashed opacity-30">
                                    <span class="tech-label text-[10px]">"NO_RUNS_YET"</span>
                                </div>
                            }.into_any();
                        }
                        view! {
                            <RunHistoryTable
                                runs=rs
                                selected_run=selected_run
                                on_select=move |id| set_selected_run.set(Some(id))
                            />
                        }.into_any()
                    }}
                </Transition>
            </div>

            // ── Run details (when a run is selected) ──────────────────────────────
            {move || {
                if selected_run.get().is_none() {
                    return view! { <div></div> }.into_any();
                }
                view! {
                    <div class="space-y-2">
                        <span class="tech-label opacity-40">"RUN_DETAILS"</span>
                        <Transition fallback=|| view! {
                            <p class="tech-label animate-pulse text-[10px]">"LOADING_RUN..."</p>
                        }>
                            {move || match run_detail.get() {
                                None => view! { <div></div> }.into_any(),
                                Some(None) => view! {
                                    <p class="tech-label opacity-30 text-[10px]">"RUN_NOT_FOUND"</p>
                                }.into_any(),
                                Some(Some(run)) => view! {
                                    <RunDetails run=run />
                                }.into_any(),
                            }}
                        </Transition>
                    </div>
                }.into_any()
            }}

        </div>
    }
}

#[component]
fn DatasetRow<F>(
    dataset: EvaluationDatasetSummaryDto,
    is_active: F,
    on_select: impl Fn() + Send + Sync + 'static,
) -> impl IntoView
where
    F: Fn() -> bool + Send + Sync + 'static,
{
    let on_select_stored = StoredValue::new(on_select);
    let (status_cls, status_label) = eval_status_display(&dataset.status);
    view! {
        <button
            class=move || format!(
                "w-full text-left px-3 py-2 rounded border text-xs font-mono transition-colors {}",
                if is_active() {
                    "border-[var(--color-accent)] bg-[var(--color-accent)]/10"
                } else {
                    "border-[var(--color-border)] hover:border-[var(--color-accent)]/50"
                }
            )
            on:click=move |_| on_select_stored.with_value(|f| f())
        >
            <div class="flex items-center justify-between">
                <span>{dataset.label}</span>
                <div class="flex gap-3 items-center">
                    <span class=format!("font-bold tracking-widest {status_cls}")>{status_label}</span>
                    <span class="opacity-50">{format!("{}Q", dataset.question_count)}</span>
                </div>
            </div>
        </button>
    }
}

#[component]
fn RunHistoryTable(
    runs: Vec<EvaluationRunSummaryDto>,
    selected_run: ReadSignal<Option<Uuid>>,
    on_select: impl Fn(Uuid) + Send + Sync + 'static,
) -> impl IntoView {
    let on_select_stored = StoredValue::new(on_select);
    view! {
        <div class="card-outer overflow-hidden">
            <table class="w-full text-xs border-collapse">
                <thead>
                    <tr class="bg-[var(--color-card-inner)]/50">
                        <th class="text-left px-3 py-2 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "RUN_ID"
                        </th>
                        <th class="text-left px-3 py-2 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "STATUS"
                        </th>
                        <th class="text-right px-3 py-2 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "VARIANTS"
                        </th>
                        <th class="text-right px-3 py-2 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "CREATED"
                        </th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-[var(--color-border)]">
                    {runs.into_iter().map(|r| {
                        let rid = r.run_id;
                        let is_selected = move || selected_run.get() == Some(rid);
                        let (status_cls, status_label) = eval_status_display(&r.status);
                        let date_short = r.created_at.get(..16).unwrap_or(&r.created_at).to_string();
                        let run_short = r.run_id.to_string()[..8].to_string();
                        view! {
                            <tr
                                class=move || format!(
                                    "cursor-pointer transition-colors {}",
                                    if is_selected() {
                                        "bg-[var(--color-accent)]/10"
                                    } else {
                                        "hover:bg-[var(--color-card-inner)]/50"
                                    }
                                )
                                on:click=move |_| on_select_stored.with_value(|f| f(rid))
                            >
                                <td class="px-3 py-2 font-mono opacity-60">
                                    {format!("{run_short}…")}
                                </td>
                                <td class=format!("px-3 py-2 font-bold tracking-widest {status_cls}")>
                                    {status_label}
                                </td>
                                <td class="px-3 py-2 text-right opacity-60">
                                    {r.variant_count.to_string()}
                                </td>
                                <td class="px-3 py-2 text-right opacity-40">{date_short}</td>
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn RunDetails(run: crate::shared::EvaluationRunDto) -> impl IntoView {
    let (status_cls, status_label) = eval_status_display(&run.status);
    view! {
        <div class="card-outer p-4 space-y-4">
            <div class="flex items-center gap-4">
                <span class="font-mono text-xs opacity-50">{run.run_id.to_string()}</span>
                <span class=format!("tech-label text-[10px] font-bold {status_cls}")>
                    {status_label}
                </span>
            </div>

            {if run.variants.is_empty() {
                view! {
                    <p class="tech-label opacity-30 text-[10px]">
                        "NO_VARIANT_RESULTS — run may still be in progress."
                    </p>
                }.into_any()
            } else {
                view! { <VariantResultsTable variants=run.variants /> }.into_any()
            }}
        </div>
    }
}

#[component]
fn VariantResultsTable(variants: Vec<EvaluationVariantResult>) -> impl IntoView {
    view! {
        <div class="overflow-x-auto">
            <table class="w-full text-xs border-collapse">
                <thead>
                    <tr class="bg-[var(--color-card-inner)]/50">
                        <th class="text-left px-3 py-2 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "VARIANT"
                        </th>
                        <th class="text-right px-3 py-2 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "RECALL"
                        </th>
                        <th class="text-right px-3 py-2 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "PRECISION"
                        </th>
                        <th class="text-right px-3 py-2 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "IOU"
                        </th>
                        <th class="text-right px-3 py-2 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "Ω-PREC"
                        </th>
                        <th class="text-right px-3 py-2 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "SPLIT"
                        </th>
                        <th class="text-right px-3 py-2 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "SEL"
                        </th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-[var(--color-border)]">
                    {variants.into_iter().map(|v| {
                        let is_selected = v.selected;
                        view! {
                            <tr class=move || if is_selected {
                                "bg-emerald-500/5 hover:bg-emerald-500/10 transition-colors"
                            } else {
                                "hover:bg-[var(--color-card-inner)]/50 transition-colors"
                            }>
                                <td class="px-3 py-2 font-mono">{v.variant.label.clone()}</td>
                                <td class="px-3 py-2 text-right font-mono">
                                    {format!("{:.1}%", v.metrics.recall_mean * 100.0)}
                                </td>
                                <td class="px-3 py-2 text-right font-mono">
                                    {format!("{:.1}%", v.metrics.precision_mean * 100.0)}
                                </td>
                                <td class="px-3 py-2 text-right font-mono">
                                    {format!("{:.1}%", v.metrics.iou_mean * 100.0)}
                                </td>
                                <td class="px-3 py-2 text-right font-mono">
                                    {format!("{:.1}%", v.metrics.precision_omega_mean * 100.0)}
                                </td>
                                <td class="px-3 py-2 text-right opacity-50 uppercase">
                                    {v.split.as_str()}
                                </td>
                                <td class=format!(
                                    "px-3 py-2 text-right font-bold {}",
                                    if is_selected { "text-emerald-400" } else { "opacity-20" }
                                )>
                                    {if is_selected { "✓" } else { "·" }}
                                </td>
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }
}

fn eval_status_display(status: &str) -> (&'static str, &'static str) {
    match status {
        "completed" => ("text-emerald-500/80", "COMPLETED"),
        "failed" => ("text-red-500/80", "FAILED"),
        "running" => ("text-blue-400/80", "RUNNING"),
        "generating" => ("text-blue-400/80", "GENERATING"),
        "pending" => ("text-amber-500/80", "PENDING"),
        _ => ("opacity-50", "UNKNOWN"),
    }
}
