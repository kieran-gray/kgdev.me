use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;

use crate::components::log_panel::LogPanel;
use crate::server_fns::{get_post_detail, start_ingest};
use crate::shared::{IngestOptions, LogEvent, LogLevel, PostDetailDto};

#[component]
pub fn PostDetailPage() -> impl IntoView {
    let params = use_params_map();
    let slug = Memo::new(move |_| params.with(|p| p.get("slug").unwrap_or_default().to_string()));

    let detail = Resource::new(
        move || slug.get(),
        move |s| async move {
            if s.is_empty() {
                return Err("missing slug".to_string());
            }
            get_post_detail(s).await.map_err(|e| e.to_string())
        },
    );

    view! {
        <div class="space-y-6">
            <Suspense fallback=|| view! { <p class="tech-label animate-pulse">"INITIALIZING_COMPONENT..."</p> }>
                {move || {
                    detail
                        .get()
                        .map(|res| match res {
                            Ok(d) => view! { <PostDetailView detail=d /> }.into_any(),
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
            </Suspense>
        </div>
    }
}

#[component]
fn PostDetailView(detail: PostDetailDto) -> impl IntoView {
    let slug = StoredValue::new(detail.slug.clone());
    let (events, set_events) = signal::<Vec<LogEvent>>(Vec::new());
    let (running, set_running) = signal(false);

    let trigger = move |options: IngestOptions| {
        if running.get_untracked() {
            return;
        }
        set_running.set(true);
        set_events.set(vec![LogEvent {
            level: LogLevel::Info,
            message: format!(
                "INIT_PROCESS: force_mode={}, dry_run={}...",
                options.force, options.dry_run
            ),
        }]);
        let slug_value = slug.get_value();
        spawn_local(async move {
            match start_ingest(slug_value, options).await {
                Ok(info) => {
                    open_event_stream(info.stream_url, set_events, set_running);
                }
                Err(e) => {
                    set_events.update(|evs| {
                        evs.push(LogEvent {
                            level: LogLevel::Error,
                            message: format!("PROCESS_FAILURE: {e}"),
                        });
                    });
                    set_running.set(false);
                }
            }
        });
    };

    let dirty_badge = if detail.is_dirty {
        view! { <span class="badge text-amber-500 border-amber-500">"DIRTY"</span> }.into_any()
    } else if detail.manifest_post_version.is_some() {
        view! { <span class="badge text-emerald-500 border-emerald-500">"STABLE"</span> }.into_any()
    } else {
        view! { <span class="badge text-amber-500 border-amber-500">"UNINITIALIZED"</span> }
            .into_any()
    };

    let title = detail.title.clone();
    let slug_disp = detail.slug.clone();
    let current_v = detail.current_post_version.clone();
    let manifest_v = detail
        .manifest_post_version
        .clone()
        .unwrap_or_else(|| "N/A".to_string());
    let chunk_count = detail
        .manifest_chunk_count
        .map(|c| c.to_string())
        .unwrap_or_else(|| "00".into());
    let ingested_at = detail
        .manifest_ingested_at
        .clone()
        .unwrap_or_else(|| "NEVER".into());
    let body_len = detail.markdown_body_length;
    let glossary = detail.glossary_terms.clone();
    let chunks = detail.chunk_preview.clone();

    view! {
        <div class="space-y-2 border-b border-[var(--color-border)] pb-4">
            <div class="flex items-center gap-4">
                {dirty_badge}
            </div>
            <h1 class="text-3xl font-bold tracking-tight uppercase">{title}</h1>
            <p class="text-xs font-mono opacity-50">{format!("./posts/{}", slug_disp)}</p>
        </div>

        <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-0 border-x border-t border-[var(--color-border)]">
            <Stat label="BODY_SIZE" value=format!("{} B", body_len) />
            <Stat label="GLOSSARY_NODES" value=format!("{:02}", glossary.len()) />
            <Stat label="MANIFEST_CHUNKS" value=format!("{:02}", chunk_count.parse::<i32>().unwrap_or(0)) />
            <Stat label="LAST_SYNC" value=ingested_at />
            <Stat label="HEAD_HASH" value=short_hash(&current_v) />
            <Stat label="MANIFEST_HASH" value=short_hash(&manifest_v) />
        </div>

        <div class="grid grid-cols-1 lg:grid-cols-3 gap-6 pt-6">
            <div class="lg:col-span-1 space-y-6">
                <section class="card-outer p-4 space-y-4">
                    <div class="flex flex-col">
                        <span class="tech-label">"action.panel"</span>
                        <h2 class="text-lg font-bold">"INGEST_CONTROLS"</h2>
                    </div>
                    <div class="grid grid-cols-1 gap-2">
                        <button
                            class="btn btn-primary w-full justify-center"
                            disabled=move || running.get()
                            on:click=move |_| trigger(IngestOptions { force: false, dry_run: false })
                        >
                            "EXECUTE_INGEST"
                        </button>
                        <button
                            class="btn w-full justify-center"
                            disabled=move || running.get()
                            on:click=move |_| trigger(IngestOptions { force: true, dry_run: false })
                        >
                            "FORCE_REBUILD"
                        </button>
                        <button
                            class="btn w-full justify-center"
                            disabled=move || running.get()
                            on:click=move |_| trigger(IngestOptions { force: false, dry_run: true })
                        >
                            "DRY_RUN"
                        </button>
                    </div>
                </section>

                <section class="card-outer p-4 space-y-4">
                    <div class="flex flex-col">
                        <span class="tech-label">"metadata.glossary"</span>
                        <h2 class="text-lg font-bold">{format!("TERMS [{:02}]", glossary.len())}</h2>
                    </div>
                    {if glossary.is_empty() {
                        view! {
                            <p class="tech-label opacity-50">"NO_TERMS_REFERENCED"</p>
                        }
                            .into_any()
                    } else {
                        view! {
                            <ul class="space-y-3">
                                {glossary
                                    .into_iter()
                                    .map(|g| {
                                        view! {
                                            <li class="card-inner p-2 border-l-2 border-l-[var(--color-accent)]">
                                                <div class="flex justify-between items-start mb-1">
                                                    <span class="font-bold text-xs uppercase">{g.term}</span>
                                                    <span class="tech-label opacity-40">{g.slug}</span>
                                                </div>
                                                <p class="text-[10px] leading-relaxed opacity-70">
                                                    {g.definition_excerpt}
                                                </p>
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        }
                            .into_any()
                    }}
                </section>
            </div>

            <div class="lg:col-span-2 space-y-6">
                <section class="card-outer p-4 space-y-4 min-h-[400px] flex flex-col">
                    <div class="flex flex-col">
                        <span class="tech-label">"process.output"</span>
                        <h2 class="text-lg font-bold">"CONSOLE_LOG"</h2>
                    </div>
                    <div class="flex-1 bg-black/20">
                        <LogPanel events=events />
                    </div>
                </section>

                <section class="card-outer p-4 space-y-4">
                    <div class="flex flex-col">
                        <span class="tech-label">"data.preview"</span>
                        <h2 class="text-lg font-bold">{format!("CHUNK_STREAM [{:02}]", chunks.len())}</h2>
                    </div>
                    <div class="space-y-4">
                        {chunks
                            .into_iter()
                            .map(|c| {
                                let prefix = if c.is_glossary { "GLOSSARY" } else { "POST_BODY" };
                                view! {
                                    <div class="card-inner p-3 relative overflow-hidden group">
                                        <div class="absolute top-0 right-0 px-2 py-0.5 bg-[var(--color-border)] tech-label opacity-50">
                                            {format!("ID:{}", c.chunk_id)}
                                        </div>
                                        <div class="flex flex-col mb-2">
                                            <span class="tech-label text-[var(--color-accent)]">{prefix}</span>
                                            <span class="font-bold text-sm uppercase tracking-tight">{c.heading}</span>
                                        </div>
                                        <pre class="log-pre text-[10px] bg-transparent border-none p-0 max-height-[10rem]">
                                            {c.text_excerpt}
                                        </pre>
                                        <div class="mt-2 flex justify-between items-center tech-label opacity-40">
                                            <span>{format!("LENGTH: {} CHARS", c.text_length)}</span>
                                        </div>
                                    </div>
                                }
                            })
                            .collect_view()}
                    </div>
                </section>
            </div>
        </div>
    }
}

#[component]
fn Stat(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="p-3 border-r border-b border-[var(--color-border)] bg-[var(--color-card-inner)]/50">
            <div class="tech-label opacity-50 mb-1">{label}</div>
            <div class="font-mono text-xs font-bold truncate tracking-wider">{value}</div>
        </div>
    }
}

fn short_hash(hash: &str) -> String {
    if hash.len() <= 12 {
        hash.to_string()
    } else {
        format!("{}...", &hash[..12])
    }
}

#[cfg(feature = "hydrate")]
fn open_event_stream(
    url: String,
    set_events: WriteSignal<Vec<LogEvent>>,
    set_running: WriteSignal<bool>,
) {
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::{EventSource, MessageEvent};

    let source = match EventSource::new(&url) {
        Ok(s) => s,
        Err(e) => {
            set_events.update(|evs| {
                evs.push(LogEvent {
                    level: LogLevel::Error,
                    message: format!("failed to open event stream: {:?}", e),
                });
            });
            set_running.set(false);
            return;
        }
    };

    let source_for_close = source.clone();
    let set_events_msg = set_events;
    let set_running_msg = set_running;
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |evt: MessageEvent| {
        let data = evt.data().as_string().unwrap_or_default();
        if data == "__done__" {
            set_running_msg.set(false);
            source_for_close.close();
            return;
        }
        match serde_json::from_str::<LogEvent>(&data) {
            Ok(e) => set_events_msg.update(|evs| evs.push(e)),
            Err(err) => set_events_msg.update(|evs| {
                evs.push(LogEvent {
                    level: LogLevel::Error,
                    message: format!("malformed log event: {err}"),
                });
            }),
        }
    });
    source.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();

    let source_for_err = source.clone();
    let on_error = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        set_running.set(false);
        source_for_err.close();
    });
    source.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    on_error.forget();
}

#[cfg(not(feature = "hydrate"))]
fn open_event_stream(
    _url: String,
    _set_events: WriteSignal<Vec<LogEvent>>,
    _set_running: WriteSignal<bool>,
) {
}
