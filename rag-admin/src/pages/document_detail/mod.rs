use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;

mod chunk_card;
mod chunks_tab;
mod dataset_detail;
mod eval_launcher;
mod eval_parser;
mod evaluation_tab;
mod indexings_tab;
mod redirect_by_id;
mod run_detail;
mod source_tab;
mod utils;

use chunks_tab::ChunksTab;
pub use dataset_detail::DatasetDetailPage;
use evaluation_tab::EvaluationTab;
use indexings_tab::IndexingsTab;
pub use redirect_by_id::DocumentByIdRedirect;
pub use run_detail::RunDetailPage;
use source_tab::SourceTab;
use utils::short_hash;

use crate::components::event_bus::use_invalidator;
use crate::components::primitives::{EmptyState, PageHeader, Status, StatusPill, Surface};
use crate::server_functions::configuration::{
    get_chunking_configurations, get_pipeline_configurations,
};
use crate::server_functions::source_document::{
    get_document_detail_by_source_ref, import_source_document,
};
use crate::shared::{
    aggregate_type, ChunkingConfigurationDto, PipelineConfigurationDto, SourceDocumentDetailDto,
    SourceDocumentDto,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Evaluation,
    Indexings,
    Source,
    Chunks,
}

/// Document detail page — generic across all source document types.
///
/// Route: `/documents/{doc_type}/{source_ref}`
///
/// The page has four tabs, defaulting to Evaluation (the primary happy-path
/// per the UX plan). Adding a new document type only requires registering an
/// adapter in `AppState` and extending the small label helpers in `utils`.
#[component]
pub fn DocumentDetailPage() -> impl IntoView {
    let params = use_params_map();
    let source_ref =
        Memo::new(move |_| params.with(|p| p.get("source_ref").unwrap_or_default().to_string()));

    // Live-refetch the document detail whenever a relevant event arrives.
    let doc_invalidator = use_invalidator(|e| {
        e.from_any(&[aggregate_type::SOURCE_DOCUMENT, aggregate_type::INDEXING])
    });
    let detail = Resource::new(
        move || (source_ref.get(), doc_invalidator.get()),
        move |(slug, _)| async move {
            if slug.is_empty() {
                return Err("missing source_ref".to_string());
            }
            get_document_detail_by_source_ref(slug)
                .await
                .map_err(|e| e.to_string())
        },
    );

    // Pipelines are slow-moving config but still eventful — refetch when the
    // Configuration aggregate or any of its sub-aggregates emits an event.
    let pipeline_invalidator = use_invalidator(|e| e.from_any(&[aggregate_type::CONFIGURATION]));
    let pipelines = Resource::new(
        move || pipeline_invalidator.get(),
        |_| async move { get_pipeline_configurations().await.unwrap_or_default() },
    );
    let chunking_configurations = Resource::new(
        move || pipeline_invalidator.get(),
        |_| async move { get_chunking_configurations().await.unwrap_or_default() },
    );

    view! {
        <div>
            <Transition fallback=|| view! {
                <p class="muted">"Loading document…"</p>
            }>
                {move || {
                    let pipelines = pipelines.get().unwrap_or_default();
                    let chunking_configurations = chunking_configurations.get().unwrap_or_default();
                    detail.get().map(|res| match res {
                        Err(e) => view! {
                            <Surface>
                                <div class="log-line-error">{format!("Failed to load: {e}")}</div>
                            </Surface>
                        }.into_any(),
                        Ok(None) => view! {
                            <UnregisteredDocument source_ref=source_ref.get() />
                        }.into_any(),
                        Ok(Some(existing)) => view! {
                            <DocumentWorkspace
                                detail=existing
                                pipelines=pipelines
                                chunking_configurations=chunking_configurations
                                source_ref=source_ref.get()
                            />
                        }.into_any(),
                    })
                }}
            </Transition>
        </div>
    }
}

#[component]
fn DocumentWorkspace(
    detail: SourceDocumentDetailDto,
    pipelines: Vec<PipelineConfigurationDto>,
    chunking_configurations: Vec<ChunkingConfigurationDto>,
    source_ref: String,
) -> impl IntoView {
    let (active_tab, set_active_tab) = signal(Tab::Evaluation);
    let detail_stored = StoredValue::new(detail.clone());
    let pipelines_stored = StoredValue::new(pipelines);
    let chunking_stored = StoredValue::new(chunking_configurations);
    let source_ref_stored = StoredValue::new(source_ref.clone());

    let (header_eyebrow, header_title, header_subtitle, header_status) =
        derive_header(&detail.document, &detail.indexings);
    let (status_kind, status_label) = header_status;

    view! {
        <div>
            <PageHeader
                title=header_title
                eyebrow=header_eyebrow
                subtitle=header_subtitle.unwrap_or_default()
                actions=Box::new(move || view! {
                    <StatusPill label=status_label.clone() kind=status_kind />
                }.into_any())
            />

            <nav class="border-b border-[var(--color-border)] mb-6 flex gap-1">
                <TabButton label="Evaluation"
                    active=move || active_tab.get() == Tab::Evaluation
                    on_click=Box::new(move || set_active_tab.set(Tab::Evaluation)) />
                <TabButton label="Indexings"
                    active=move || active_tab.get() == Tab::Indexings
                    on_click=Box::new(move || set_active_tab.set(Tab::Indexings)) />
                <TabButton label="Source"
                    active=move || active_tab.get() == Tab::Source
                    on_click=Box::new(move || set_active_tab.set(Tab::Source)) />
                <TabButton label="Chunks"
                    active=move || active_tab.get() == Tab::Chunks
                    on_click=Box::new(move || set_active_tab.set(Tab::Chunks)) />
            </nav>

            {move || match active_tab.get() {
                Tab::Evaluation => view! {
                    <EvaluationTab
                        detail=Some(detail_stored.get_value())
                        pipelines=pipelines_stored.get_value()
                        chunking_configurations=chunking_stored.get_value()
                    />
                }.into_any(),
                Tab::Indexings => view! {
                    <IndexingsTab
                        detail=Some(detail_stored.get_value())
                        pipelines=pipelines_stored.get_value()
                        source_ref=source_ref_stored.get_value()
                    />
                }.into_any(),
                Tab::Source => view! {
                    <SourceTab source_ref=source_ref_stored.get_value() />
                }.into_any(),
                Tab::Chunks => view! {
                    <ChunksTab detail=Some(detail_stored.get_value()) />
                }.into_any(),
            }}
        </div>
    }
}

#[component]
fn TabButton(
    label: &'static str,
    active: impl Fn() -> bool + Send + Sync + 'static,
    on_click: Box<dyn Fn() + Send + Sync>,
) -> impl IntoView {
    let on_click_stored = StoredValue::new(on_click);
    view! {
        <button
            type="button"
            class=move || format!(
                "px-4 py-2.5 -mb-px border-b-2 text-sm font-medium transition-colors {}",
                if active() {
                    "border-[var(--color-accent)] text-text"
                } else {
                    "border-transparent muted hover:text-text"
                }
            )
            on:click=move |_| on_click_stored.with_value(|f| f())
        >
            {label}
        </button>
    }
}

fn derive_header(
    doc: &SourceDocumentDto,
    indexings: &[crate::shared::IndexingDto],
) -> (String, String, Option<String>, (Status, String)) {
    let type_label = document_type_label(&doc.document_type);
    let eyebrow = format!("Documents / {} / {}", type_label, doc.source_ref_key);
    let title = doc.title.clone();
    let subtitle = Some(format!(
        "{type_label} · v{} · {}",
        doc.latest_version,
        short_hash(&doc.latest_content_hash),
    ));
    let status = derive_status(indexings);
    (eyebrow, title, subtitle, status)
}

fn derive_status(indexings: &[crate::shared::IndexingDto]) -> (Status, String) {
    if indexings.is_empty() {
        return (Status::Info, "Registered".to_string());
    }
    if indexings.iter().any(|i| i.status.contains("Indexed")) {
        return (Status::Ok, "Indexed".to_string());
    }
    if indexings.iter().any(|i| i.status.contains("Failed")) {
        return (Status::Fail, "Failed".to_string());
    }
    if indexings.iter().any(|i| i.status.contains("Embedding")) {
        return (Status::Info, "Embedded".to_string());
    }
    if indexings.iter().any(|i| i.status.contains("Chunking")) {
        return (Status::Info, "Chunked".to_string());
    }
    (Status::Pending, "Pending".to_string())
}

fn document_type_label(doc_type: &str) -> &'static str {
    match doc_type {
        "BlogPost" => "Blog post",
        _ => "Document",
    }
}

#[component]
fn UnregisteredDocument(source_ref: String) -> impl IntoView {
    let source_ref_stored = StoredValue::new(source_ref.clone());
    let (busy, set_busy) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    let import = move |_| {
        if busy.get_untracked() {
            return;
        }
        let slug = source_ref_stored.get_value();
        set_busy.set(true);
        set_error.set(None);
        spawn_local(async move {
            match import_source_document(slug).await {
                Ok(_) => {
                    // The detail Resource is keyed on the source_ref signal +
                    // SourceDocument event invalidator; the import dispatches
                    // SourceDocument events so the parent will refetch.
                    set_busy.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("{e}")));
                    set_busy.set(false);
                }
            }
        });
    };

    view! {
        <div>
            <PageHeader
                title=source_ref.clone()
                eyebrow=format!("Documents / {source_ref}")
                subtitle="This document is available upstream but hasn't been imported yet. Import it to inspect chunks, run experiments, or index it.".to_string()
            />
            <Surface>
                <EmptyState
                    title="Not imported"
                    body="Importing fetches the upstream content and registers a versioned SourceDocument. After that you can run evaluations, chunk it with different strategies, and (optionally) index it.".to_string()
                    action=Box::new(move || view! {
                        <div class="flex flex-col items-start gap-2">
                            <button
                                type="button"
                                class="btn btn-primary"
                                disabled=busy
                                on:click=import
                            >
                                {move || if busy.get() { "Importing…" } else { "Import from source" }}
                            </button>
                            {move || error.get().map(|e| view! {
                                <div class="log-line-error text-sm">{e}</div>
                            })}
                        </div>
                    }.into_any())
                />
            </Surface>
        </div>
    }
}
