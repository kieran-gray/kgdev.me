use leptos::prelude::*;
use leptos_router::components::A;

use crate::server_functions::source_document::list_documents_with_status;
use crate::shared::DocumentListItemDto;

#[component]
pub fn PostsListPage() -> impl IntoView {
    let docs = Resource::new(|| (), |_| async move { list_documents_with_status().await });

    view! {
        <div class="space-y-8">
            <div class="px-6 flex flex-col gap-1">
                <span class="tech-label opacity-40">"SYSTEM_VIEW / SOURCE_DOCUMENTS"</span>
                <h1 class="text-3xl font-bold tracking-tight">"DOCUMENT_INDEX"</h1>
            </div>

            <Suspense fallback=|| {
                view! {
                    <div class="px-6">
                        <p class="tech-label animate-pulse">"LOADING_DATA..."</p>
                    </div>
                }
            }>
                {move || {
                    docs.get()
                        .map(|res| match res {
                            Ok(list) => view! { <DocumentsTable docs=list /> }.into_any(),
                            Err(e) => {
                                view! {
                                    <div class="px-6">
                                        <div class="card-outer p-4 log-line-error font-mono text-sm">
                                            {format!("ERROR_LOG: {e}")}
                                        </div>
                                    </div>
                                }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn DocumentsTable(docs: Vec<DocumentListItemDto>) -> impl IntoView {
    if docs.is_empty() {
        return view! {
            <div class="px-6">
                <div class="card-outer p-8 text-center tech-label opacity-40">
                    "NO_RECORDS_FOUND"
                </div>
            </div>
        }
            .into_any();
    }

    let total = docs.len();
    view! {
        <div class="border-y border-[var(--color-border)] bg-black/10 overflow-hidden">
            <table class="w-full text-sm border-collapse">
                <thead>
                    <tr class="bg-[var(--color-card-inner)]/50">
                        <th class="text-left px-6 py-3 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "REF"
                        </th>
                        <th class="text-left px-4 py-3 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "TITLE"
                        </th>
                        <th class="text-left px-4 py-3 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "TYPE"
                        </th>
                        <th class="text-left px-4 py-3 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "STATUS"
                        </th>
                        <th class="text-right px-4 py-3 tech-label opacity-50 border-b border-[var(--color-border)]">
                            "VERSION"
                        </th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-[var(--color-border)]">
                    {docs
                        .into_iter()
                        .map(|doc| {
                            let href = format!(
                                "/documents/{}/{}",
                                doc.document_type.to_lowercase(),
                                doc.source_ref_key,
                            );
                            let (status_label, status_cls) = ingest_status(&doc);
                            let version_label = doc
                                .latest_version
                                .map(|v| format!("v{v}"))
                                .unwrap_or_else(|| "—".into());
                            let type_label = document_type_label(&doc.document_type);
                            view! {
                                <tr class="hover:bg-[var(--color-accent)]/5 transition-colors group">
                                    <td class="px-6 py-3 font-mono text-xs">
                                        <A
                                            href=href
                                            attr:class="text-[var(--color-accent)] hover:underline"
                                        >
                                            {format!("./{}", doc.source_ref_key)}
                                        </A>
                                    </td>
                                    <td class="px-4 py-3 font-medium text-sm">{doc.title}</td>
                                    <td class="px-4 py-3">
                                        <span class="text-[10px] font-bold tracking-widest opacity-50">
                                            {type_label}
                                        </span>
                                    </td>
                                    <td class="px-4 py-3">
                                        <span class=format!(
                                            "text-[10px] font-bold tracking-widest {}",
                                            status_cls,
                                        )>{status_label}</span>
                                    </td>
                                    <td class="px-4 py-3 font-mono text-xs text-right opacity-40">
                                        {version_label}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
            <div class="px-6 py-2 bg-black/40 flex justify-between items-center border-t border-[var(--color-border)]">
                <span class="tech-label opacity-40 text-[9px]">
                    {format!("TOTAL_RECORDS: {:03}", total)}
                </span>
                <span class="tech-label opacity-40 text-[9px]">"SYSTEM_STABLE // PAGE_01"</span>
            </div>
        </div>
    }
        .into_any()
}

fn ingest_status(doc: &DocumentListItemDto) -> (&'static str, &'static str) {
    if doc.document_id.is_none() {
        ("NEVER_INGESTED", "text-amber-500/80")
    } else if doc.indexings.is_empty() {
        // Has a document record but no indexings
        ("REGISTERED", "text-blue-400/80")
    } else if doc.indexings.iter().any(|i| i.status.contains("Indexed")) {
        ("INDEXED", "text-emerald-500/80")
    } else if doc.indexings.iter().any(|i| i.status.contains("Failed")) {
        ("FAILED", "text-red-500/80")
    } else {
        ("IN_PROGRESS", "text-amber-500/80")
    }
}

fn document_type_label(doc_type: &str) -> &'static str {
    match doc_type {
        "BlogPost" => "BLOG_POST",
        _ => "DOCUMENT",
    }
}
