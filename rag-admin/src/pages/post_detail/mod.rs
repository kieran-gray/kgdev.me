use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;

mod chunk_card;
mod evaluation_dialog;
mod evaluation_results;
mod tuning_panel;
mod utils;

use chunk_card::ChunkCard;
use evaluation_dialog::EvaluationDialog;
use evaluation_results::EvaluationResults;
use tuning_panel::TuningPanel;
use utils::{open_event_stream, short_hash, truncate_chars};

use crate::components::log_panel::LogPanel;
use crate::server_fns::{
    clear_post_chunking_config, get_latest_evaluation_result, get_post_detail, start_ingest,
};
use crate::shared::{
    ChunkStrategy, ChunkingConfig, EvaluationRunResult, IngestOptions, LogEvent, LogLevel,
    PostDetailDto,
};

const GLOSSARY_DEFINITION_PREVIEW_CHARS: usize = 300;

#[component]
pub fn PostDetailPage() -> impl IntoView {
    let params = use_params_map();
    let slug = Memo::new(move |_| params.with(|p| p.get("slug").unwrap_or_default().to_string()));

    let (override_config, set_override_config) = signal::<Option<ChunkingConfig>>(None);

    let detail = Resource::new(
        move || (slug.get(), override_config.get()),
        move |(s, ov)| async move {
            if s.is_empty() {
                return Err("missing slug".to_string());
            }
            get_post_detail(s, ov).await.map_err(|e| e.to_string())
        },
    );

    view! {
        <div class="space-y-6">
            <Transition fallback=|| view! { <p class="tech-label animate-pulse">"INITIALIZING_COMPONENT..."</p> }>
                {move || {
                    detail
                        .get()
                        .map(|res| match res {
                            Ok(d) => view! {
                                <PostDetailView
                                    detail=d
                                    override_config=override_config
                                    set_override_config=set_override_config
                                />
                            }.into_any(),
                            Err(e) => {
                                view! {
                                    <div class="card-outer p-4 log-line-error font-mono text-sm">
                                        {format!("SYSTEM_FAULT: {e}")}
                                    </div>
                                }
                                    .into_any()
                            }
                        })
                }}
            </Transition>
        </div>
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Overview,
    Refinement,
    Chunks,
}

#[component]
fn PostDetailView(
    detail: PostDetailDto,
    override_config: ReadSignal<Option<ChunkingConfig>>,
    set_override_config: WriteSignal<Option<ChunkingConfig>>,
) -> impl IntoView {
    let slug = StoredValue::new(detail.slug.clone());
    let (active_tab, set_active_tab) = signal(Tab::Refinement);

    let (ingest_events, set_ingest_events) = signal::<Vec<LogEvent>>(Vec::new());
    let (ingest_running, set_ingest_running) = signal(false);
    let (ingest_dialog_open, set_ingest_dialog_open) = signal(false);
    let (pending_ingest_options, set_pending_ingest_options) =
        signal::<Option<IngestOptions>>(None);

    let (eval_dialog_open, set_eval_dialog_open) = signal(false);
    let (eval_result, set_eval_result) =
        signal::<Option<Result<EvaluationRunResult, String>>>(None);
    let (save_status, set_save_status) = signal::<Option<(bool, String)>>(None);

    let default_chunking = detail.default_chunking;
    let effective_chunking = detail.effective_chunking;
    let token_limit = detail.embedding_token_limit;

    let make_ingest_options = move |force: bool, dry_run: bool| IngestOptions {
        force,
        dry_run,
        chunking_override: override_config.get(),
    };

    let open_ingest_dialog = move |options: IngestOptions| {
        set_pending_ingest_options.set(Some(options));
        set_ingest_events.set(Vec::new());
        set_ingest_dialog_open.set(true);
    };

    let confirm_ingest = move |_| {
        let Some(options) = pending_ingest_options.get_untracked() else {
            return;
        };
        if ingest_running.get_untracked() {
            return;
        }
        set_ingest_running.set(true);
        set_ingest_events.set(vec![LogEvent {
            level: LogLevel::Info,
            message: format!(
                "INIT_PROCESS: force_mode={}, dry_run={}, override={}...",
                options.force,
                options.dry_run,
                if options.chunking_override.is_some() {
                    "yes"
                } else {
                    "no"
                }
            ),
        }]);
        let slug_value = slug.get_value();
        spawn_local(async move {
            match start_ingest(slug_value, options).await {
                Ok(info) => {
                    open_event_stream(info.stream_url, set_ingest_events, set_ingest_running);
                }
                Err(e) => {
                    set_ingest_events.update(|evs| {
                        evs.push(LogEvent {
                            level: LogLevel::Error,
                            message: format!("PROCESS_FAILURE: {e}"),
                        });
                    });
                    set_ingest_running.set(false);
                }
            }
        });
    };

    let clear_saved = move |_| {
        set_save_status.set(None);
        let slug = slug.get_value();
        spawn_local(async move {
            match clear_post_chunking_config(slug).await {
                Ok(()) => {
                    set_override_config.set(None);
                    set_save_status.set(Some((true, "POST_CONFIG_CLEARED".into())));
                }
                Err(e) => set_save_status.set(Some((false, format!("CLEAR_FAULT: {e}")))),
            }
        });
    };

    let dirty_badge = if detail.is_dirty {
        view! { <span class="badge !text-amber-500 !border-amber-500 !bg-amber-900/60">"DIRTY"</span> }.into_any()
    } else if detail.manifest_post_version.is_some() {
        view! { <span class="badge !text-emerald-500 !border-emerald-500 !bg-emerald-900/60">"UP TO DATE"</span> }.into_any()
    } else {
        view! { <span class="badge text-amber-500 border-amber-500">"UNINITIALIZED"</span> }
            .into_any()
    };

    let body_len = detail.markdown_body_length;
    let glossary = StoredValue::new(detail.glossary_terms.clone());
    let chunks = StoredValue::new(detail.chunk_preview.clone());
    let size_limit = effective_chunking.size_limit_for_display(token_limit);
    let title = detail.title.clone();
    let slug_disp = detail.slug.clone();
    let evaluation_slug = detail.slug.clone();

    let saved_result_slug = detail.slug.clone();
    Effect::new(move |_| {
        if eval_result.get_untracked().is_some() {
            return;
        }
        let slug = saved_result_slug.clone();
        spawn_local(async move {
            match get_latest_evaluation_result(slug).await {
                Ok(Some(result)) => {
                    if eval_result.get_untracked().is_none() {
                        set_eval_result.set(Some(Ok(result)));
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    if eval_result.get_untracked().is_none() {
                        set_eval_result.set(Some(Err(format!("LOAD_SAVED_EVALUATION_FAULT: {e}"))));
                    }
                }
            }
        });
    });

    view! {
        <Show when=move || ingest_dialog_open.get()>
            <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
                <div class="card-outer p-6 w-full max-w-2xl mx-4 flex flex-col gap-4 max-h-[80vh]">
                    <div class="flex items-start justify-between">
                        <div class="flex flex-col">
                            <span class="tech-label">"process.confirm"</span>
                            <h2 class="text-lg font-bold">
                                {move || match pending_ingest_options.get() {
                                    Some(IngestOptions { force: true, .. }) => "FORCE_REBUILD",
                                    Some(IngestOptions { dry_run: true, .. }) => "DRY_RUN",
                                    _ => "EXECUTE_INGEST",
                                }}
                            </h2>
                        </div>
                        <button
                            class="tech-label opacity-50 hover:opacity-100 px-2 py-0.5 border border-[var(--color-border)] cursor-pointer disabled:cursor-not-allowed disabled:opacity-20"
                            disabled=move || ingest_running.get()
                            on:click=move |_| set_ingest_dialog_open.set(false)
                        >
                            "✕"
                        </button>
                    </div>

                    <div class="flex-1 overflow-auto bg-black/20 min-h-[200px]">
                        <LogPanel events=ingest_events />
                    </div>

                    <div class="flex justify-end gap-2">
                        {move || {
                            if ingest_running.get() {
                                view! {
                                    <span class="tech-label opacity-50 animate-pulse">"PROCESS_RUNNING..."</span>
                                }.into_any()
                            } else if ingest_events.get().is_empty() {
                                view! {
                                    <div class="flex gap-2">
                                        <button class="btn" on:click=move |_| set_ingest_dialog_open.set(false)>"CANCEL"</button>
                                        <button class="btn btn-primary" on:click=confirm_ingest>"CONFIRM"</button>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <button class="btn" on:click=move |_| set_ingest_dialog_open.set(false)>"CLOSE"</button>
                                }.into_any()
                            }
                        }}
                    </div>
                </div>
            </div>
        </Show>

        <EvaluationDialog
            slug=evaluation_slug
            current_config=effective_chunking
            open=eval_dialog_open
            set_open=set_eval_dialog_open
            set_eval_result=set_eval_result
        />

        <div class="space-y-4">
            <div class="flex flex-col justify-between gap-4 border-b border-[var(--color-border)] pb-4">
                <div class="space-y-1">
                    <div class="flex items-center gap-3">
                        {dirty_badge}
                        <span class="tech-label opacity-40">{format!("./posts/{}", slug_disp)}</span>
                    </div>
                    <h1 class="text-3xl font-bold tracking-tight uppercase">{title}</h1>
                </div>

                <div class="flex gap-2">
                    <button
                        class="btn btn-primary px-6"
                        on:click=move |_| open_ingest_dialog(make_ingest_options(false, false))
                    >
                        "EXECUTE_INGEST"
                    </button>
                    <button
                        class="btn"
                        on:click=move |_| open_ingest_dialog(make_ingest_options(true, false))
                    >
                        "FORCE"
                    </button>
                </div>
            </div>

            <div class="flex flex-wrap gap-6 py-2 px-4 bg-[var(--color-card-inner)]/30 border border-[var(--color-border)]">
                <MiniStat label="BODY" value=format!("{} B", body_len) />
                <MiniStat label="GLOSSARY" value=format!("{:02}", glossary.with_value(|g| g.len())) />
                <MiniStat label="CHUNKS" value=detail.manifest_chunk_count.map(|c| c.to_string()).unwrap_or_else(|| "0".into()) />
                <MiniStat label="HASH" value=short_hash(&detail.current_post_version) />
            </div>

            <div class="flex gap-1 border-b border-[var(--color-border)]">
                <TabButton
                    label="REFINEMENT"
                    active=move || active_tab.get() == Tab::Refinement
                    on_click=Box::new(move || set_active_tab.set(Tab::Refinement))
                />
                <TabButton
                    label="CHUNKS"
                    active=move || active_tab.get() == Tab::Chunks
                    on_click=Box::new(move || set_active_tab.set(Tab::Chunks))
                />
                <TabButton
                    label="METADATA"
                    active=move || active_tab.get() == Tab::Overview
                    on_click=Box::new(move || set_active_tab.set(Tab::Overview))
                />
            </div>

            <div class="pt-4">
                {move || match active_tab.get() {
                    Tab::Refinement => view! {
                        <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                            <div class="lg:col-span-1 space-y-6">
                                <section class="card-outer p-4 space-y-4">
                                    <div class="flex flex-col">
                                        <span class="tech-label">"action.evaluation"</span>
                                        <h2 class="text-lg font-bold">"EVALUATION_LAB"</h2>
                                        <p class="tech-label opacity-50 mt-1">
                                            "Open the laboratory to generate synthetic datasets and run performance sweeps."
                                        </p>
                                    </div>
                                    <button
                                        class="btn btn-primary w-full justify-center"
                                        on:click=move |_| set_eval_dialog_open.set(true)
                                    >
                                        "OPEN_EVALUATION_LAB"
                                    </button>
                                    <button
                                        class="btn w-full justify-center"
                                        on:click=clear_saved
                                    >
                                        "CLEAR_SAVED_CONFIG"
                                    </button>
                                    {move || {
                                        save_status
                                            .get()
                                            .map(|(ok, msg)| {
                                                let cls = if ok { "text-emerald-500" } else { "text-red-500" };
                                                view! { <div class=format!("tech-label mt-2 {}", cls)>{msg}</div> }
                                            })
                                    }}
                                </section>

                                <TuningPanel
                                    default_config=default_chunking
                                    committed=override_config
                                    set_committed=set_override_config
                                />

                            </div>

                            <div class="lg:col-span-2 space-y-6">
                                {move || {
                                    eval_result
                                        .get()
                                        .map(|res| match res {
                                            Ok(result) => view! {
                                                <section class="card-outer p-4 space-y-4">
                                                    <EvaluationResults
                                                        result=result
                                                        slug=slug.get_value()
                                                        set_override_config=set_override_config
                                                        set_save_status=set_save_status
                                                    />
                                                </section>
                                            }.into_any(),
                                            Err(e) => view! {
                                                <div class="tech-label log-line-error card-outer p-4">
                                                    {format!("EVALUATION_FAULT: {e}")}
                                                </div>
                                            }.into_any(),
                                        })
                                        .unwrap_or_else(|| view! {
                                            <div class="card-outer p-8 flex flex-col items-center justify-center border-dashed opacity-50">
                                                <span class="tech-label mb-2">"NO_RECENT_EVALUATION"</span>
                                                <p class="text-xs text-center">"Run an evaluation in the laboratory to see comparative metrics here."</p>
                                            </div>
                                        }.into_any())
                                }}
                            </div>
                        </div>
                    }.into_any(),

                    Tab::Chunks => view! {
                        <div class="space-y-4">
                            <div class="flex items-center justify-between px-2">
                                <div class="flex flex-col">
                                    <span class="tech-label">"data.preview"</span>
                                    <h2 class="text-lg font-bold uppercase">{move || format!("CHUNK_STREAM [{:02}]", chunks.with_value(|c| c.len()))}</h2>
                                    <span class="tech-label opacity-50 mt-1">
                                        {strategy_label(effective_chunking, size_limit)}
                                    </span>
                                </div>
                                {move || override_config.get().is_some().then(|| view! {
                                    <span class="badge !text-amber-400 !border-amber-400">"USING_OVERRIDE_PREVIEW"</span>
                                })}
                            </div>
                            <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                                {chunks
                                    .with_value(|c| {
                                        c.clone()
                                            .into_iter()
                                            .map(|c| view! { <ChunkCard chunk=c strategy=effective_chunking.strategy size_limit=size_limit /> })
                                            .collect_view()
                                    })}
                            </div>
                        </div>
                    }.into_any(),

                    Tab::Overview => view! {
                        <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                            <div class="lg:col-span-1 space-y-6">
                                <section class="card-outer p-4 space-y-4">
                                    <div class="flex flex-col">
                                        <span class="tech-label">"metadata.stats"</span>
                                        <h2 class="text-lg font-bold">"SYSTEM_METADATA"</h2>
                                    </div>
                                    <div class="space-y-3">
                                        <DetailField label="MANIFEST_VERSION" value=detail.manifest_post_version.clone().unwrap_or_else(|| "N/A".into()) />
                                        <DetailField label="INGESTED_AT" value=detail.manifest_ingested_at.clone().unwrap_or_else(|| "NEVER".into()) />
                                        <DetailField label="EMBEDDING_LIMIT" value=format!("{} TOKENS", token_limit) />
                                    </div>
                                </section>
                            </div>
                            <div class="lg:col-span-2 space-y-6">
                                <section class="card-outer p-4 space-y-4">
                                    <div class="flex flex-col">
                                        <span class="tech-label">"metadata.glossary"</span>
                                        <h2 class="text-lg font-bold">{move || format!("TERMS [{:02}]", glossary.with_value(|g| g.len()))}</h2>
                                    </div>
                                    {move || if glossary.with_value(|g| g.is_empty()) {
                                        view! { <p class="tech-label opacity-50">"NO_TERMS_REFERENCED"</p> }.into_any()
                                    } else {
                                        view! {
                                            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                                {glossary
                                                    .with_value(|g| {
                                                        g.clone()
                                                            .into_iter()
                                                            .map(|g| view! {
                                                                <div class="card-inner p-3 border-l-2 border-l-[var(--color-accent)]">
                                                                    <div class="flex justify-between items-start mb-1">
                                                                        <span class="font-bold text-xs uppercase">{g.term}</span>
                                                                        <span class="tech-label opacity-40">{g.slug}</span>
                                                                    </div>
                                                                    <p class="text-[10px] leading-relaxed opacity-70">
                                                                        {truncate_chars(&g.definition, GLOSSARY_DEFINITION_PREVIEW_CHARS)}
                                                                    </p>
                                                                </div>
                                                            })
                                                            .collect_view()
                                                    })}
                                            </div>
                                        }.into_any()
                                    }}
                                </section>
                            </div>
                        </div>
                    }.into_any(),
                }}
            </div>
        </div>
    }
}

#[component]
fn TabButton<F>(
    label: &'static str,
    active: F,
    on_click: Box<dyn Fn() + Send + Sync>,
) -> impl IntoView
where
    F: Fn() -> bool + Send + Sync + 'static,
{
    let cls = move || {
        if active() {
            "px-6 py-2 border-b-2 border-[var(--color-accent)] tech-label font-bold text-[var(--color-accent)]"
        } else {
            "px-6 py-2 border-b-2 border-transparent tech-label opacity-50 hover:opacity-100 hover:border-[var(--color-border)]"
        }
    };

    let on_click_stored = StoredValue::new(on_click);
    view! {
        <button class=cls on:click=move |_| on_click_stored.with_value(|f| f())>
            {label}
        </button>
    }
}

#[component]
fn MiniStat(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="flex flex-col">
            <span class="tech-label opacity-40 text-[9px] uppercase">{label}</span>
            <span class="font-mono text-[11px] font-bold">{value}</span>
        </div>
    }
}

#[component]
fn DetailField(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="flex justify-between items-center py-1 border-b border-[var(--color-border)]/30">
            <span class="tech-label opacity-50">{label}</span>
            <span class="font-mono text-xs truncate max-w-[200px]">{value}</span>
        </div>
    }
}

fn strategy_label(c: ChunkingConfig, size_limit: u32) -> String {
    match c.strategy {
        ChunkStrategy::Bert => format!(
            "STRATEGY: BERT · TOKEN_LIMIT: {} · TARGET: {} · OVERLAP: {} · MIN: {}",
            size_limit, c.target_chars, c.overlap_chars, c.min_chars
        ),
        ChunkStrategy::Llm => format!(
            "STRATEGY: LLM · TOKEN_LIMIT: {} · TARGET: {} · OVERLAP: {} · MIN: {}",
            size_limit, c.target_chars, c.overlap_chars, c.min_chars
        ),
        ChunkStrategy::Section => format!("STRATEGY: SECTION · MAX_CHARS: {}", c.max_section_chars),
    }
}
