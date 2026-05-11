use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

use crate::components::event_bus::use_invalidator;
use crate::components::log_panel::LogPanel;
use crate::components::primitives::{EmptyState, Status, StatusPill, Surface};
use crate::server_functions::source_document::{
    request_indexing, start_chunking_stage, start_embedding_stage, start_source_document_ingest,
    start_upsert_stage,
};
use crate::shared::{
    aggregate_type, ChunkingConfig, ChunkingConfigurationDto, IndexingDto, IngestJobInfo, LogEvent,
    LogLevel, PipelineConfigurationDto, SourceDocumentDetailDto,
};

use super::utils::open_event_stream;

/// "Indexings" tab — table of all indexing aggregates for this document plus
/// an inline launcher for a new one. Each row exposes manual per-stage
/// controls (Chunk → Embed → Upsert) so the operator can advance the pipeline
/// step by step, or use the one-shot "Run full" button on the new-indexing
/// panel to do everything in one go.
#[component]
pub fn IndexingsTab(
    detail: Option<SourceDocumentDetailDto>,
    pipelines: Vec<PipelineConfigurationDto>,
    source_ref: String,
) -> impl IntoView {
    let pipelines_stored = StoredValue::new(pipelines);
    let source_ref_stored = StoredValue::new(source_ref);

    // Pull live chunking configurations so the launcher can pick a named
    // config rather than hard-coded inline params.
    let invalidator =
        use_invalidator(|e| e.from_any(&[aggregate_type::CONFIGURATION, aggregate_type::INDEXING]));
    let chunking_configurations = Resource::new(
        move || invalidator.get(),
        |_| async move {
            crate::server_functions::configuration::get_chunking_configurations()
                .await
                .unwrap_or_default()
        },
    );

    let indexings = detail.map(|d| d.indexings).unwrap_or_default();

    view! {
        <div class="grid grid-cols-1 lg:grid-cols-[1fr_360px] gap-6">
            <div>
                <Surface flush=true>
                    {if indexings.is_empty() {
                        view! {
                            <div class="p-6">
                                <EmptyState
                                    title="No indexings yet"
                                    body="Pick a pipeline and chunking configuration on the right to set up the first indexing.".to_string()
                                />
                            </div>
                        }.into_any()
                    } else {
                        view! { <IndexingsTable indexings=indexings /> }.into_any()
                    }}
                </Surface>
            </div>
            <div>
                <Transition fallback=|| view! { <p class="muted">"Loading chunking library…"</p> }>
                    {move || chunking_configurations.get().map(|chunking_list| view! {
                        <NewIndexingPanel
                            pipelines=pipelines_stored
                            source_ref=source_ref_stored
                            chunking_configurations=StoredValue::new(chunking_list)
                        />
                    })}
                </Transition>
            </div>
        </div>
    }
}

#[component]
fn IndexingsTable(indexings: Vec<IndexingDto>) -> impl IntoView {
    view! {
        <table class="data-table">
            <thead>
                <tr>
                    <th>"Pipeline"</th>
                    <th>"Status"</th>
                    <th>"Stage controls"</th>
                    <th class="text-right">"Doc version"</th>
                </tr>
            </thead>
            <tbody>
                {indexings.into_iter().map(|ix| {
                    let (kind, label) = status_display(&ix.status);
                    let pipeline_short = ix.pipeline_configuration_id.to_string()[..8].to_string();
                    view! {
                        <tr>
                            <td>
                                <span class="font-mono text-xs muted">
                                    {format!("{pipeline_short}…")}
                                </span>
                            </td>
                            <td><StatusPill label=label.to_string() kind=kind /></td>
                            <td>
                                <StageControls ix=ix.clone() />
                            </td>
                            <td class="text-right font-mono text-xs muted">
                                {format!("v{}", ix.document_version)}
                            </td>
                        </tr>
                    }
                }).collect_view()}
            </tbody>
        </table>
    }
}

/// Per-row stage advancers. Each button calls the corresponding server fn
/// and opens a log stream inline. Enabled/disabled state is driven by the
/// IndexingDto status string from the read model.
#[component]
fn StageControls(ix: IndexingDto) -> impl IntoView {
    let indexing_id = ix.indexing_id;
    let status = ix.status.clone();
    let (running_stage, set_running_stage) = signal::<Option<&'static str>>(None);
    let (log_events, set_log_events) = signal::<Vec<LogEvent>>(Vec::new());

    let make_runner = move |stage: &'static str, runner: fn(Uuid) -> _| {
        move |_| {
            if running_stage.get_untracked().is_some() {
                return;
            }
            set_running_stage.set(Some(stage));
            set_log_events.set(vec![]);
            let fut = runner(indexing_id);
            spawn_local(async move {
                match fut.await {
                    Ok(IngestJobInfo { stream_url, .. }) => {
                        // open_event_stream flips set_running back to false on
                        // job completion; we mirror that into running_stage.
                        let (proxy_running, set_proxy_running) = signal(true);
                        open_event_stream(stream_url, set_log_events, set_proxy_running);
                        // Watch proxy_running and clear running_stage when done.
                        Effect::new(move |_| {
                            if !proxy_running.get() {
                                set_running_stage.set(None);
                            }
                        });
                    }
                    Err(e) => {
                        set_running_stage.set(None);
                        set_log_events.update(|evs| {
                            evs.push(LogEvent {
                                level: LogLevel::Error,
                                message: format!("{e}"),
                            });
                        });
                    }
                }
            });
        }
    };

    let chunk_done =
        status.contains("Chunking") || status.contains("Embedding") || status.contains("Indexed");
    let embed_done = status.contains("Embedding") || status.contains("Indexed");
    let indexed_done = status.contains("Indexed");

    let chunk_busy = move || running_stage.get() == Some("chunk");
    let embed_busy = move || running_stage.get() == Some("embed");
    let upsert_busy = move || running_stage.get() == Some("upsert");
    let any_busy = move || running_stage.get().is_some();

    let on_chunk = make_runner("chunk", |id| {
        Box::pin(start_chunking_stage(id))
            as std::pin::Pin<
                Box<
                    dyn std::future::Future<
                            Output = Result<IngestJobInfo, leptos::prelude::ServerFnError>,
                        > + Send,
                >,
            >
    });
    let on_embed = make_runner("embed", |id| {
        Box::pin(start_embedding_stage(id))
            as std::pin::Pin<
                Box<
                    dyn std::future::Future<
                            Output = Result<IngestJobInfo, leptos::prelude::ServerFnError>,
                        > + Send,
                >,
            >
    });
    let on_upsert = make_runner("upsert", |id| {
        Box::pin(start_upsert_stage(id))
            as std::pin::Pin<
                Box<
                    dyn std::future::Future<
                            Output = Result<IngestJobInfo, leptos::prelude::ServerFnError>,
                        > + Send,
                >,
            >
    });

    view! {
        <div class="flex flex-col gap-1.5">
            <div class="flex gap-1.5">
                <StageButton
                    label="Chunk"
                    done=chunk_done
                    busy=Signal::derive(chunk_busy)
                    any_busy=Signal::derive(any_busy)
                    on_click=Box::new(on_chunk)
                />
                <StageButton
                    label="Embed"
                    done=embed_done
                    busy=Signal::derive(embed_busy)
                    any_busy=Signal::derive(any_busy)
                    disabled_reason=Box::new(move || (!chunk_done).then(|| "Chunk first".to_string()))
                    on_click=Box::new(on_embed)
                />
                <StageButton
                    label="Index"
                    done=indexed_done
                    busy=Signal::derive(upsert_busy)
                    any_busy=Signal::derive(any_busy)
                    disabled_reason=Box::new(move || (!embed_done).then(|| "Embed first".to_string()))
                    on_click=Box::new(on_upsert)
                />
            </div>
            {move || (!log_events.with(|e| e.is_empty())).then(|| view! {
                <div class="h-32 overflow-y-auto border border-[var(--color-border)] rounded">
                    <LogPanel events=log_events />
                </div>
            })}
        </div>
    }
}

#[component]
fn StageButton(
    label: &'static str,
    done: bool,
    #[prop(into)] busy: Signal<bool>,
    #[prop(into)] any_busy: Signal<bool>,
    #[prop(optional)] disabled_reason: Option<Box<dyn Fn() -> Option<String> + Send + Sync>>,
    on_click: Box<dyn Fn(leptos::ev::MouseEvent) + Send + Sync>,
) -> impl IntoView {
    let disabled_reason_stored = StoredValue::new(disabled_reason);
    let blocked = move || disabled_reason_stored.with_value(|f| f.as_ref().and_then(|f| f()));
    let on_click_stored = StoredValue::new(on_click);
    view! {
        <button
            type="button"
            class=move || {
                let base = "btn text-xs px-2 py-1";
                if done {
                    format!("{base} btn-ghost")
                } else if busy.get() {
                    format!("{base} btn-primary")
                } else if blocked().is_some() || any_busy.get() {
                    format!("{base} opacity-50")
                } else {
                    format!("{base} btn-primary")
                }
            }
            disabled=move || any_busy.get() || blocked().is_some()
            title=move || blocked().unwrap_or_default()
            on:click=move |ev| on_click_stored.with_value(|f| f(ev))
        >
            {move || if busy.get() {
                format!("{label}…")
            } else if done {
                format!("{label} ✓")
            } else {
                label.to_string()
            }}
        </button>
    }
}

#[component]
fn NewIndexingPanel(
    pipelines: StoredValue<Vec<PipelineConfigurationDto>>,
    source_ref: StoredValue<String>,
    chunking_configurations: StoredValue<Vec<ChunkingConfigurationDto>>,
) -> impl IntoView {
    let (active_pipeline, set_active_pipeline) = signal::<Option<Uuid>>(None);
    let (active_chunking, set_active_chunking) = signal::<Option<Uuid>>(None);
    let (running, set_running) = signal(false);
    let (log_events, set_log_events) = signal::<Vec<LogEvent>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);

    // Pre-select first chunking config on mount so the form has a sane default.
    Effect::new(move |_| {
        if active_chunking.get_untracked().is_none() {
            if let Some(first) = chunking_configurations
                .with_value(|c| c.first().map(|cc| cc.chunking_configuration_id))
            {
                set_active_chunking.set(Some(first));
            }
        }
    });

    let resolve_chunking_config = move || -> Option<ChunkingConfig> {
        let id = active_chunking.get()?;
        chunking_configurations.with_value(|c| {
            c.iter()
                .find(|cc| cc.chunking_configuration_id == id)
                .map(|cc| cc.config)
        })
    };

    let can_start = move || {
        active_pipeline.get().is_some() && active_chunking.get().is_some() && !running.get()
    };

    let on_create_pending = move |_| {
        let (Some(pipeline_id), Some(config)) = (active_pipeline.get(), resolve_chunking_config())
        else {
            return;
        };
        let slug = source_ref.get_value();
        set_running.set(true);
        set_error.set(None);
        set_log_events.set(vec![]);
        spawn_local(async move {
            match request_indexing(slug, pipeline_id, config).await {
                Ok(_indexing_id) => {
                    // Indexing event invalidates the document detail Resource
                    // upstream; the new row appears automatically.
                    set_running.set(false);
                }
                Err(e) => {
                    set_running.set(false);
                    set_error.set(Some(format!("{e}")));
                }
            }
        });
    };

    let on_run_full = move |_| {
        let (Some(pipeline_id), Some(config)) = (active_pipeline.get(), resolve_chunking_config())
        else {
            return;
        };
        let slug = source_ref.get_value();
        set_running.set(true);
        set_error.set(None);
        set_log_events.set(vec![]);
        spawn_local(async move {
            match start_source_document_ingest(slug, pipeline_id, config).await {
                Ok(IngestJobInfo { stream_url, .. }) => {
                    open_event_stream(stream_url, set_log_events, set_running);
                }
                Err(e) => {
                    set_running.set(false);
                    set_error.set(Some(format!("{e}")));
                }
            }
        });
    };

    view! {
        <Surface title="New indexing".to_string()>
            <div class="space-y-4">
                <div>
                    <div class="eyebrow mb-2">"Pipeline"</div>
                    <div class="space-y-1.5">
                        {move || {
                            let ps = pipelines.get_value();
                            if ps.is_empty() {
                                return view! {
                                    <p class="muted text-sm">
                                        "No pipelines configured — add one in Pipelines."
                                    </p>
                                }.into_any();
                            }
                            ps.into_iter().map(|pc| {
                                let pc_id = pc.pipeline_configuration_id;
                                let active = move || active_pipeline.get() == Some(pc_id);
                                view! {
                                    <button
                                        type="button"
                                        class=move || format!(
                                            "w-full text-left px-3 py-2 rounded border text-sm transition-colors {}",
                                            if active() {
                                                "border-[var(--color-accent)] text-[var(--color-accent)] bg-[var(--color-accent-soft)]"
                                            } else {
                                                "border-[var(--color-border)] hover:border-[var(--color-border-strong)]"
                                            }
                                        )
                                        on:click=move |_| set_active_pipeline.set(Some(pc_id))
                                    >
                                        {pc.name}
                                    </button>
                                }
                            }).collect_view().into_any()
                        }}
                    </div>
                </div>

                <div>
                    <div class="eyebrow mb-2">"Chunking configuration"</div>
                    {move || {
                        let cs = chunking_configurations.get_value();
                        if cs.is_empty() {
                            return view! {
                                <p class="muted text-sm">
                                    "No chunking configurations — add some in Chunking."
                                </p>
                            }.into_any();
                        }
                        view! {
                            <select
                                class="input"
                                on:change=move |e| {
                                    let v = event_target_value(&e);
                                    set_active_chunking.set(Uuid::parse_str(&v).ok());
                                }
                            >
                                {cs.into_iter().map(|cc| {
                                    let id = cc.chunking_configuration_id;
                                    let label = format!("{} · {}", cc.name, cc.config.describe());
                                    let selected = active_chunking.get() == Some(id);
                                    view! {
                                        <option value=id.to_string() selected=selected>{label}</option>
                                    }
                                }).collect_view()}
                            </select>
                        }.into_any()
                    }}
                </div>

                <div class="flex flex-col gap-2 pt-2">
                    <button
                        type="button"
                        class="btn btn-primary w-full justify-center"
                        disabled=move || !can_start()
                        on:click=on_create_pending
                    >
                        {move || if running.get() {
                            "Working…"
                        } else if active_pipeline.get().is_none() {
                            "Select a pipeline"
                        } else if active_chunking.get().is_none() {
                            "Select a chunking config"
                        } else {
                            "Create — manual stages"
                        }}
                    </button>
                    <button
                        type="button"
                        class="btn w-full justify-center"
                        disabled=move || !can_start()
                        on:click=on_run_full
                    >
                        "Run full ingest"
                    </button>
                    <p class="text-xs faint">
                        "Manual stages registers the indexing in Pending so you can run chunk · embed · index step by step.
                         Run full does everything in one go."
                    </p>
                </div>

                {move || error.get().map(|e| view! {
                    <div class="log-line-error text-sm">{e}</div>
                })}

                {move || (running.get() || !log_events.with(|e| e.is_empty())).then(|| view! {
                    <div class="mt-2">
                        <div class="eyebrow mb-1">"Live log"</div>
                        <div class="h-44 overflow-y-auto">
                            <LogPanel events=log_events />
                        </div>
                    </div>
                })}
            </div>
        </Surface>
    }
}

fn status_display(status: &str) -> (Status, &'static str) {
    if status.contains("Indexed") {
        (Status::Ok, "Indexed")
    } else if status.contains("Failed") {
        (Status::Fail, "Failed")
    } else if status.contains("Embedding") {
        (Status::Pending, "Embedded")
    } else if status.contains("Chunking") {
        (Status::Pending, "Chunked")
    } else {
        (Status::Pending, "Pending")
    }
}
