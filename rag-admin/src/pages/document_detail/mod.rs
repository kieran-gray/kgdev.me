use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;

mod chunk_card;
mod utils;

use chunk_card::ChunkCard;
use utils::{open_event_stream, short_hash};

use crate::components::log_panel::LogPanel;
use crate::server_functions::configuration::get_pipeline_configurations;
use crate::server_functions::source_document::{
    get_chunks, get_document_detail_by_source_ref, start_source_document_ingest,
};
use crate::shared::{
    ChunkingConfig, IndexingDto, IngestJobInfo, LogEvent, PipelineConfigurationDto,
    SectionChunkingConfig, SourceDocumentDetailDto,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Ingest,
    Chunks,
}

/// Document detail page — generic across all source document types.
/// Route: /documents/{doc_type}/{source_ref}
///
/// To add support for a new document type:
///   1. Register an adapter in AppState (infrastructure layer)
///   2. Add a match arm in `document_type_label()` and `DocumentTypeMetadata`
///      for the new type's specific display
#[component]
pub fn DocumentDetailPage() -> impl IntoView {
    let params = use_params_map();
    let source_ref =
        Memo::new(move |_| params.with(|p| p.get("source_ref").unwrap_or_default().to_string()));

    // Existing ingest record — None if document was never ingested.
    let detail = Resource::new(
        move || source_ref.get(),
        move |slug| async move {
            if slug.is_empty() {
                return Err("missing source_ref".to_string());
            }
            get_document_detail_by_source_ref(slug)
                .await
                .map_err(|e| e.to_string())
        },
    );

    // Pipeline configurations are always needed for the ingest panel.
    let pipelines = Resource::new(
        || (),
        |_| async move { get_pipeline_configurations().await.unwrap_or_default() },
    );

    view! {
        <div class="space-y-6">
            <Transition fallback=|| {
                view! { <p class="tech-label animate-pulse px-6">"INITIALIZING_COMPONENT..."</p> }
            }>
                {move || {
                    let pipelines = pipelines.get().unwrap_or_default();
                    detail
                        .get()
                        .map(|res| match res {
                            Err(e) => view! {
                                <div class="px-6 card-outer p-4 log-line-error font-mono text-sm">
                                    {format!("SYSTEM_FAULT: {e}")}
                                </div>
                            }
                                .into_any(),
                            Ok(existing_detail) => view! {
                                <DocumentView
                                    detail=existing_detail
                                    pipelines=pipelines
                                    source_ref=source_ref.get()
                                />
                            }
                                .into_any(),
                        })
                }}
            </Transition>
        </div>
    }
}

/// Main view — shows header (if ingested) + always-visible ingest panel + log stream.
#[component]
fn DocumentView(
    /// None when never ingested; Some once ingest has run at least once.
    detail: Option<SourceDocumentDetailDto>,
    pipelines: Vec<PipelineConfigurationDto>,
    source_ref: String,
) -> impl IntoView {
    let source_ref_stored = StoredValue::new(source_ref.clone());
    let pipelines_stored = StoredValue::new(pipelines);

    let (active_tab, set_active_tab) = signal(Tab::Ingest);

    let (log_events, set_log_events) = signal::<Vec<LogEvent>>(Vec::new());
    let (ingest_running, set_ingest_running) = signal(false);
    let (active_pipeline, set_active_pipeline) = signal::<Option<uuid::Uuid>>(None);
    let (chunking_config, set_chunking_config) =
        signal(ChunkingConfig::Section(SectionChunkingConfig {
            max_section_tokens: 512,
        }));

    // Refresh detail after a successful ingest by invalidating the resource.
    let detail_stored = StoredValue::new(detail.clone());

    view! {
        <div class="space-y-6 px-6">
            // ── Header ────────────────────────────────────────────────────────────
            {match detail.clone() {
                Some(d) => view! { <DocumentHeader doc=d.document /> }.into_any(),
                None => view! {
                    <div class="flex flex-col gap-1">
                        <span class="tech-label opacity-40">"SYSTEM_VIEW / DOCUMENT_DETAIL"</span>
                        <h1 class="text-3xl font-bold tracking-tight">{source_ref.clone()}</h1>
                        <p class="tech-label text-amber-500/60 text-[10px]">
                            "NOT_YET_INGESTED — select a pipeline below to begin"
                        </p>
                    </div>
                }
                    .into_any(),
            }}

            <div class="border-b border-[var(--color-border)]">
                <div class="flex gap-1">
                    <TabButton
                        label="INGEST_CONTROL"
                        active=move || active_tab.get() == Tab::Ingest
                        on_click=Box::new(move || set_active_tab.set(Tab::Ingest))
                    />
                    <TabButton
                        label="CHUNK_EXPLORER"
                        active=move || active_tab.get() == Tab::Chunks
                        on_click=Box::new(move || set_active_tab.set(Tab::Chunks))
                    />
                </div>
            </div>

            {move || match active_tab.get() {
                Tab::Ingest => view! {
                    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 animate-in fade-in duration-200">
                        // ── Left: Pipeline selector + ingest trigger ──────────────────────
                        <div class="space-y-4">
                            <div>
                                <span class="tech-label opacity-40">"INGEST_CONTROL"</span>
                                <h2 class="text-lg font-bold mt-1">"PIPELINE_INGEST"</h2>
                            </div>

                            <div class="card-outer p-4 space-y-4">
                                // Pipeline buttons
                                <div class="space-y-1">
                                    <span class="tech-label opacity-50 text-[10px]">"1. SELECT_PIPELINE"</span>
                                    <div class="flex flex-col gap-2 mt-1">
                                        {move || {
                                            let ps = pipelines_stored.get_value();
                                            if ps.is_empty() {
                                                return view! {
                                                    <p class="tech-label opacity-30 text-xs">
                                                        "No pipelines configured — add one in PIPELINE_CONFIG"
                                                    </p>
                                                }
                                                    .into_any();
                                            }
                                            ps.into_iter()
                                                .map(|pc| {
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
                                                                },
                                                            )
                                                            on:click=move |_| set_active_pipeline.set(Some(pc_id))
                                                        >
                                                            <span class="opacity-40 mr-1">"▶"</span>
                                                            {pc.name}
                                                        </button>
                                                    }
                                                })
                                                .collect_view()
                                                .into_any()
                                        }}
                                    </div>
                                </div>

                                // Chunking strategy quick-select
                                <div class="space-y-1">
                                    <span class="tech-label opacity-50 text-[10px]">"2. CHUNKING_CONFIG"</span>
                                    <div class="flex gap-2 flex-wrap mt-1">
                                        {["256", "384", "512"].into_iter().map(|t| {
                                            let tokens: u32 = t.parse().unwrap_or(512);
                                            let current = move || match chunking_config.get() {
                                                ChunkingConfig::Section(c) => c.max_section_tokens,
                                                _ => 0,
                                            };
                                            view! {
                                                <button
                                                    class=move || format!(
                                                        "px-2 py-1 text-[10px] font-mono tracking-widest border rounded transition-colors {}",
                                                        if current() == tokens {
                                                            "border-[var(--color-accent)] text-[var(--color-accent)]"
                                                        } else {
                                                            "border-[var(--color-border)] opacity-50 hover:opacity-100"
                                                        },
                                                    )
                                                    on:click=move |_| set_chunking_config.set(
                                                        ChunkingConfig::Section(SectionChunkingConfig {
                                                            max_section_tokens: tokens,
                                                        }),
                                                    )
                                                >
                                                    {format!("section:{t}")}
                                                </button>
                                            }
                                        }).collect_view()}
                                    </div>
                                </div>

                                // Ingest button
                                <button
                                    class="w-full py-2 text-xs font-bold tracking-widest border border-[var(--color-accent)] text-[var(--color-accent)] hover:bg-[var(--color-accent)]/10 transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
                                    disabled=move || active_pipeline.get().is_none() || ingest_running.get()
                                    on:click={
                                        move |_| {
                                            let Some(pipeline_id) = active_pipeline.get() else {
                                                return;
                                            };
                                            let slug = source_ref_stored.get_value();
                                            let config = chunking_config.get();
                                            set_log_events.set(vec![]);
                                            set_ingest_running.set(true);
                                            spawn_local(async move {
                                                match start_source_document_ingest(slug, pipeline_id, config)
                                                    .await
                                                {
                                                    Ok(IngestJobInfo { stream_url, .. }) => {
                                                        open_event_stream(stream_url, set_log_events, set_ingest_running);
                                                    }
                                                    Err(e) => {
                                                        set_ingest_running.set(false);
                                                        set_log_events.update(|evs| {
                                                            evs.push(crate::shared::LogEvent {
                                                                level: crate::shared::LogLevel::Error,
                                                                message: format!("{e}"),
                                                            });
                                                        });
                                                    }
                                                }
                                            });
                                        }
                                    }
                                >
                                    {move || {
                                        if ingest_running.get() {
                                            "INGESTING..."
                                        } else if active_pipeline.get().is_none() {
                                            "SELECT_PIPELINE_FIRST"
                                        } else {
                                            "START_INGEST"
                                        }
                                    }}
                                </button>
                            </div>

                            // Existing indexing records
                            {move || {
                                let indexings = detail_stored
                                    .get_value()
                                    .map(|d| d.indexings)
                                    .unwrap_or_default();
                                if indexings.is_empty() {
                                    view! {
                                        <p class="tech-label opacity-30 text-[10px]">
                                            "No indexings yet — run the first ingest above."
                                        </p>
                                    }
                                        .into_any()
                                } else {
                                    view! { <IndexingsTable indexings=indexings /> }.into_any()
                                }
                            }}
                        </div>

                        // ── Right: Log panel ──────────────────────────────────────────────
                        <div class="space-y-2">
                            <span class="tech-label opacity-40">"INGEST_LOG"</span>
                            <div class="h-[480px] overflow-y-auto card-outer p-3">
                                <LogPanel events=log_events />
                            </div>
                        </div>
                    </div>
                }.into_any(),

                Tab::Chunks => {
                    let indexings = detail_stored
                        .get_value()
                        .map(|d| d.indexings)
                        .unwrap_or_default();
                    view! {
                        <ChunksView indexings=indexings />
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Document header — type-specific display.
/// Add a match arm here for each new document type.
#[component]
fn DocumentHeader(doc: crate::shared::SourceDocumentDto) -> impl IntoView {
    let type_label = document_type_label(&doc.document_type);
    let hash_short = short_hash(&doc.latest_content_hash).to_string();

    view! {
        <div class="flex flex-col gap-1">
            <span class="tech-label opacity-40">
                {format!("SYSTEM_VIEW / {} / {}", type_label, doc.source_ref_key)}
            </span>
            <h1 class="text-3xl font-bold tracking-tight">{doc.title.clone()}</h1>
            <div class="flex gap-4 items-center">
                <span class="tech-label opacity-40 text-[10px]">
                    {format!("v{} · {}…", doc.latest_version, hash_short)}
                </span>

                // Type-specific metadata (add match arms for new document types)
                {match doc.document_type.as_str() {
                    "BlogPost" => view! {
                        <span class="tech-label text-[10px] opacity-40">"TYPE: BLOG_POST"</span>
                    }
                        .into_any(),
                    other => view! {
                        <span class="tech-label text-[10px] opacity-40">
                            {format!("TYPE: {}", other.to_uppercase())}
                        </span>
                    }
                        .into_any(),
                }}
            </div>
        </div>
    }
}

#[component]
fn IndexingsTable(indexings: Vec<IndexingDto>) -> impl IntoView {
    view! {
        <div class="space-y-1">
            <span class="tech-label opacity-40 text-[10px]">"EXISTING_INDEXINGS"</span>
            <div class="card-outer overflow-hidden">
                <table class="w-full text-xs border-collapse">
                    <thead>
                        <tr class="bg-[var(--color-card-inner)]/50">
                            <th class="text-left px-3 py-2 tech-label opacity-50 border-b border-[var(--color-border)]">
                                "PIPELINE"
                            </th>
                            <th class="text-left px-3 py-2 tech-label opacity-50 border-b border-[var(--color-border)]">
                                "STATUS"
                            </th>
                            <th class="text-right px-3 py-2 tech-label opacity-50 border-b border-[var(--color-border)]">
                                "v"
                            </th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-[var(--color-border)]">
                        {indexings
                            .into_iter()
                            .map(|ix| {
                                let (cls, label) = status_display(&ix.status);
                                let pipeline_short = {
                                    let s = ix.pipeline_configuration_id.to_string();
                                    s[..8].to_string()
                                };
                                view! {
                                    <tr>
                                        <td class="px-3 py-2 font-mono opacity-50">
                                            {format!("{}…", pipeline_short)}
                                        </td>
                                        <td class=format!("px-3 py-2 font-bold tracking-widest {}", cls)>
                                            {label}
                                        </td>
                                        <td class="px-3 py-2 text-right opacity-40">
                                            {format!("v{}", ix.document_version)}
                                        </td>
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

fn status_display(status: &str) -> (&'static str, &'static str) {
    if status.contains("Indexed") {
        ("text-emerald-500/80", "INDEXED")
    } else if status.contains("Failed") {
        ("text-red-500/80", "FAILED")
    } else if status.contains("Embedding") {
        ("text-blue-400/80", "EMBEDDING")
    } else if status.contains("Chunking") {
        ("text-blue-400/80", "CHUNKING")
    } else {
        ("text-amber-500/80", "PENDING")
    }
}

fn document_type_label(doc_type: &str) -> &'static str {
    match doc_type {
        "BlogPost" => "BLOG_POST",
        _ => "DOCUMENT",
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
    let on_click_stored = StoredValue::new(on_click);
    view! {
        <button
            class=move || format!(
                "px-6 py-2 border-b-2 tech-label font-bold transition-colors {}",
                if active() {
                    "border-[var(--color-accent)] text-[var(--color-accent)]"
                } else {
                    "border-transparent opacity-50 hover:opacity-100 hover:border-[var(--color-border)]"
                }
            )
            on:click=move |_| on_click_stored.with_value(|f| f())
        >
            {label}
        </button>
    }
}

#[component]
fn ChunksView(indexings: Vec<IndexingDto>) -> impl IntoView {
    let (selected_chunk_set, set_selected_chunk_set) = signal::<Option<uuid::Uuid>>(None);

    let chunks = Resource::new(
        move || selected_chunk_set.get(),
        move |cid| async move {
            match cid {
                Some(id) => get_chunks(id).await.map_err(|e| e.to_string()),
                None => Ok(vec![]),
            }
        },
    );

    view! {
        <div class="space-y-6 animate-in fade-in duration-200">
            <div class="flex flex-col gap-4">
                <div>
                    <span class="tech-label opacity-40">"DATA_SOURCE"</span>
                    <h2 class="text-lg font-bold mt-1">"SELECT_INDEXING"</h2>
                </div>

                <div class="flex gap-2 overflow-x-auto pb-2">
                    {indexings
                        .into_iter()
                        .filter(|ix| ix.chunk_set_id.is_some())
                        .map(|ix| {
                            let cid = ix.chunk_set_id.unwrap();
                            let is_active = move || selected_chunk_set.get() == Some(cid);
                            let pipeline_short = &ix.pipeline_configuration_id.to_string()[..8];
                            view! {
                                <button
                                    class=move || format!(
                                        "px-3 py-2 rounded border text-xs font-mono transition-colors whitespace-nowrap {}",
                                        if is_active() {
                                            "border-[var(--color-accent)] bg-[var(--color-accent)]/10 text-[var(--color-accent)]"
                                        } else {
                                            "border-[var(--color-border)] hover:border-[var(--color-accent)]/50"
                                        },
                                    )
                                    on:click=move |_| set_selected_chunk_set.set(Some(cid))
                                >
                                    {format!("PIPELINE:{} (v{})", pipeline_short, ix.document_version)}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </div>

            <Transition fallback=|| view! { <p class="tech-label animate-pulse">"LOADING_CHUNKS..."</p> }>
                {move || {
                    chunks.get().map(|res| match res {
                        Err(e) => view! {
                            <div class="card-outer p-4 log-line-error font-mono text-sm">
                                {format!("SYSTEM_FAULT: {e}")}
                            </div>
                        }.into_any(),
                        Ok(cs) => {
                            if cs.is_empty() {
                                view! {
                                    <div class="card-outer p-12 flex flex-col items-center justify-center border-dashed opacity-30">
                                        <span class="tech-label">"NO_CHUNKS_LOADED"</span>
                                        <p class="text-[10px] mt-1">"Select an indexing above to explore its output."</p>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                                        {cs.into_iter()
                                            .map(|c| view! { <ChunkCard chunk=c /> })
                                            .collect_view()}
                                    </div>
                                }.into_any()
                            }
                        }
                    })
                }}
            </Transition>
        </div>
    }
}
