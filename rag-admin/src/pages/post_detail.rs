use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;

use crate::components::log_panel::LogPanel;
use crate::server_fns::{get_post_detail, start_ingest};
use crate::shared::{
    ChunkPreview, ChunkStrategy, ChunkingConfig, IngestOptions, LogEvent, LogLevel, PostDetailDto,
};

const GLOSSARY_DEFINITION_PREVIEW_CHARS: usize = 300;

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();

    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

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
            <Suspense fallback=|| view! { <p class="tech-label animate-pulse">"INITIALIZING_COMPONENT..."</p> }>
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
            </Suspense>
        </div>
    }
}

#[component]
fn PostDetailView(
    detail: PostDetailDto,
    override_config: ReadSignal<Option<ChunkingConfig>>,
    set_override_config: WriteSignal<Option<ChunkingConfig>>,
) -> impl IntoView {
    let slug = StoredValue::new(detail.slug.clone());
    let (events, set_events) = signal::<Vec<LogEvent>>(Vec::new());
    let (running, set_running) = signal(false);
    let (dialog_open, set_dialog_open) = signal(false);
    let (pending_options, set_pending_options) = signal::<Option<IngestOptions>>(None);

    let default_chunking = detail.default_chunking;
    let effective_chunking = detail.effective_chunking;
    let token_limit = detail.embedding_token_limit;

    let make_options = move |force: bool, dry_run: bool| IngestOptions {
        force,
        dry_run,
        chunking_override: override_config.get(),
    };

    let open_dialog = move |options: IngestOptions| {
        set_pending_options.set(Some(options));
        set_events.set(Vec::new());
        set_dialog_open.set(true);
    };

    let confirm = move |_| {
        let Some(options) = pending_options.get_untracked() else {
            return;
        };
        if running.get_untracked() {
            return;
        }
        set_running.set(true);
        set_events.set(vec![LogEvent {
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

    let close_dialog = move |_| {
        if running.get_untracked() {
            return;
        }
        set_dialog_open.set(false);
        set_pending_options.set(None);
        set_events.set(Vec::new());
    };

    let op_label = move || match pending_options.get() {
        Some(IngestOptions { force: true, .. }) => "FORCE_REBUILD",
        Some(IngestOptions { dry_run: true, .. }) => "DRY_RUN",
        Some(_) => "EXECUTE_INGEST",
        None => "UNKNOWN_OP",
    };

    let dirty_badge = if detail.is_dirty {
        view! { <span class="badge !text-amber-500 !border-amber-500 !bg-amber-900/60">"DIRTY"</span> }.into_any()
    } else if detail.manifest_post_version.is_some() {
        view! { <span class="badge !text-emerald-500 !border-emerald-500 !bg-emerald-900/60">"UP TO DATE"</span> }.into_any()
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
    let size_limit = effective_chunking.size_limit_for_display(token_limit);

    view! {
        // Confirmation dialog overlay
        {move || dialog_open.get().then(|| view! {
            <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
                <div class="card-outer p-6 w-full max-w-2xl mx-4 flex flex-col gap-4 max-h-[80vh]">
                    <div class="flex items-start justify-between">
                        <div class="flex flex-col">
                            <span class="tech-label">"process.confirm"</span>
                            <h2 class="text-lg font-bold">{op_label}</h2>
                        </div>
                        <button
                            class="tech-label opacity-50 hover:opacity-100 px-2 py-0.5 border border-[var(--color-border)] cursor-pointer disabled:cursor-not-allowed disabled:opacity-20"
                            disabled=move || running.get()
                            on:click=close_dialog
                        >
                            "✕"
                        </button>
                    </div>

                    {move || {
                        let has_events = !events.get().is_empty();
                        if !has_events && !running.get() {
                            view! {
                                <p class="tech-label opacity-60">
                                    {move || format!("Confirm execution of {}?", op_label())}
                                </p>
                            }.into_any()
                        } else {
                            ().into_any()
                        }
                    }}

                    <div class="flex-1 overflow-auto bg-black/20 min-h-[200px]">
                        <LogPanel events=events />
                    </div>

                    <div class="flex justify-end gap-2">
                        {move || {
                            if running.get() {
                                view! {
                                    <span class="tech-label opacity-50 animate-pulse">"PROCESS_RUNNING..."</span>
                                }.into_any()
                            } else if events.get().is_empty() {
                                view! {
                                    <div class="flex gap-2">
                                        <button
                                            class="btn w-full justify-center"
                                            on:click=close_dialog
                                        >
                                            "CANCEL"
                                        </button>
                                        <button
                                            class="btn btn-primary w-full justify-center"
                                            on:click=confirm
                                        >
                                            "CONFIRM"
                                        </button>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <button
                                        class="btn w-full justify-center"
                                        on:click=close_dialog
                                    >
                                        "CLOSE"
                                    </button>
                                }.into_any()
                            }
                        }}
                    </div>
                </div>
            </div>
        })}

        <div class="space-y-2 border-b border-[var(--color-border)] pb-4">
            <div class="flex items-center gap-4">
                {dirty_badge}
            </div>
            <h1 class="text-3xl font-bold tracking-tight uppercase">{title}</h1>
            <p class="text-xs font-mono opacity-50">{format!("./posts/{}", slug_disp)}</p>
        </div>

        <div class="grid grid-cols-2 md:grid-cols-3 gap-0 border-x border-t border-[var(--color-border)]">
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
                            on:click=move |_| open_dialog(make_options(false, false))
                        >
                            "EXECUTE_INGEST"
                        </button>
                        <button
                            class="btn w-full justify-center"
                            on:click=move |_| open_dialog(make_options(true, false))
                        >
                            "FORCE_REBUILD"
                        </button>
                        <button
                            class="btn w-full justify-center"
                            on:click=move |_| open_dialog(make_options(false, true))
                        >
                            "DRY_RUN"
                        </button>
                    </div>
                </section>

                <ChunkingOverridePanel
                    default_config=default_chunking
                    committed=override_config
                    set_committed=set_override_config
                />

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
                                                    {truncate_chars(
                                                        &g.definition,
                                                        GLOSSARY_DEFINITION_PREVIEW_CHARS,
                                                    )}
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
                <section class="card-outer p-4 space-y-4">
                    <div class="flex flex-col">
                        <span class="tech-label">"data.preview"</span>
                        <h2 class="text-lg font-bold">{format!("CHUNK_STREAM [{:02}]", chunks.len())}</h2>
                        <span class="tech-label opacity-50 mt-1">
                            {strategy_label(effective_chunking, size_limit)}
                        </span>
                        {move || {
                            if override_config.get().is_some() {
                                view! {
                                    <span class="tech-label !text-amber-400 mt-1">
                                        "PREVIEW USES ONE-SHOT OVERRIDE"
                                    </span>
                                }.into_any()
                            } else {
                                ().into_any()
                            }
                        }}
                    </div>
                    <div class="space-y-4">
                        {chunks
                            .into_iter()
                            .map(|c| view! { <ChunkCard chunk=c strategy=effective_chunking.strategy size_limit=size_limit /> })
                            .collect_view()}
                    </div>
                </section>
            </div>
        </div>
    }
}

fn strategy_label(c: ChunkingConfig, size_limit: u32) -> String {
    match c.strategy {
        ChunkStrategy::Bert => format!(
            "STRATEGY: BERT · TOKEN_LIMIT: {} · TARGET: {} · OVERLAP: {} · MIN: {}",
            size_limit, c.target_chars, c.overlap_chars, c.min_chars
        ),
        ChunkStrategy::Section => format!("STRATEGY: SECTION · MAX_CHARS: {}", c.max_section_chars),
    }
}

#[component]
fn ChunkingOverridePanel(
    default_config: ChunkingConfig,
    committed: ReadSignal<Option<ChunkingConfig>>,
    set_committed: WriteSignal<Option<ChunkingConfig>>,
) -> impl IntoView {
    let initial = committed.get_untracked().unwrap_or(default_config);
    let (working, set_working) = signal(initial);

    Effect::new(move |_| {
        let next = committed.get().unwrap_or(default_config);
        if working.get_untracked() != next {
            set_working.set(next);
        }
    });

    let strategy_value = move || match working.get().strategy {
        ChunkStrategy::Bert => "bert",
        ChunkStrategy::Section => "section",
    };

    let is_overridden = move || committed.get().is_some();
    let has_unsaved_changes = move || working.get() != committed.get().unwrap_or(default_config);

    let update = move |f: Box<dyn Fn(&mut ChunkingConfig)>| {
        set_working.update(|c| f(c));
    };

    let save = move |_| {
        let next = working.get_untracked();
        if next == default_config {
            set_committed.set(None);
        } else {
            set_committed.set(Some(next));
        }
    };

    let reset = move |_| {
        set_working.set(default_config);
        set_committed.set(None);
    };

    view! {
        <section class="card-outer p-4 space-y-4">
            <div class="flex flex-col">
                <span class="tech-label">"action.tuning"</span>
                <h2 class="text-lg font-bold">"CHUNKING_OVERRIDE"</h2>
                <p class="tech-label opacity-50 mt-1">
                    "Tune chunking for this post only. Save to apply the override to preview and ingest. \
                     Ingest applies the override one-shot and forces re-embed regardless of post version."
                </p>
            </div>

            <div class="space-y-3">
                <SmallField label="STRATEGY">
                    <select
                        class="input font-mono text-xs"
                        prop:value=strategy_value
                        on:change=move |e| {
                            let v = event_target_value(&e);
                            update(Box::new(move |c| {
                                c.strategy = match v.as_str() {
                                    "section" => ChunkStrategy::Section,
                                    _ => ChunkStrategy::Bert,
                                };
                            }));
                        }
                    >
                        <option value="bert">"bert"</option>
                        <option value="section">"section"</option>
                    </select>
                </SmallField>

                {move || match working.get().strategy {
                    ChunkStrategy::Section => view! {
                        <SmallField label="MAX_SECTION_CHARS">
                            <input
                                class="input font-mono text-xs"
                                type="number"
                                min="1"
                                prop:value=move || working.get().max_section_chars.to_string()
                                on:input=move |e| {
                                    let v: u32 = event_target_value(&e).parse().unwrap_or(0);
                                    update(Box::new(move |c| c.max_section_chars = v));
                                }
                            />
                        </SmallField>
                    }.into_any(),
                    ChunkStrategy::Bert => view! {
                        <div class="space-y-3">
                            <SmallField label="TARGET_CHARS">
                                <input
                                    class="input font-mono text-xs"
                                    type="number"
                                    min="1"
                                    prop:value=move || working.get().target_chars.to_string()
                                    on:input=move |e| {
                                        let v: u32 = event_target_value(&e).parse().unwrap_or(0);
                                        update(Box::new(move |c| c.target_chars = v));
                                    }
                                />
                            </SmallField>
                            <SmallField label="OVERLAP_CHARS">
                                <input
                                    class="input font-mono text-xs"
                                    type="number"
                                    min="0"
                                    prop:value=move || working.get().overlap_chars.to_string()
                                    on:input=move |e| {
                                        let v: u32 = event_target_value(&e).parse().unwrap_or(0);
                                        update(Box::new(move |c| c.overlap_chars = v));
                                    }
                                />
                            </SmallField>
                            <SmallField label="MIN_CHARS">
                                <input
                                    class="input font-mono text-xs"
                                    type="number"
                                    min="0"
                                    prop:value=move || working.get().min_chars.to_string()
                                    on:input=move |e| {
                                        let v: u32 = event_target_value(&e).parse().unwrap_or(0);
                                        update(Box::new(move |c| c.min_chars = v));
                                    }
                                />
                            </SmallField>
                        </div>
                    }.into_any(),
                }}
            </div>

            <div class="flex flex-col items-center justify-between pt-2 border-t border-[var(--color-border)]">
                <span class=move || {
                    if has_unsaved_changes() {
                        "tech-label !text-amber-400 py-2"
                    } else if is_overridden() {
                        "tech-label !text-emerald-400 py-2"
                    } else {
                        "tech-label opacity-40 py-2"
                    }
                }>
                    {move || {
                        if has_unsaved_changes() {
                            "UNSAVED_CHANGES"
                        } else if is_overridden() {
                            "USING OVERRIDE"
                        } else {
                            "USING DEFAULT"
                        }
                    }}
                </span>
                <span class="flex gap-2">
                {move ||
                    view! {
                        <Show when=move || {has_unsaved_changes()}>
                            <button
                                type="button"
                                class="btn btn-primary"
                                disabled=move || !has_unsaved_changes()
                                on:click=save
                            >
                                "SAVE_OVERRIDE"
                            </button>
                        </Show>
                    }
                }
                {move ||
                    view! {
                        <Show when=move || {is_overridden()}>
                            <button
                                type="button"
                                class="btn"
                                disabled=move || !is_overridden() && !has_unsaved_changes()
                                on:click=reset
                            >
                                "RESET"
                            </button>
                        </Show>
                    }

                }
                </span>
            </div>
        </section>
    }
}

#[component]
fn SmallField(label: &'static str, children: Children) -> impl IntoView {
    view! {
        <label class="block space-y-1">
            <div class="tech-label opacity-70">{label}</div>
            {children()}
        </label>
    }
}

#[component]
fn Stat(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="p-3 border-r border-b border-[var(--color-border)] bg-[var(--color-card-inner)]/50 backdrop-blur-sm">
            <div class="tech-label opacity-50 mb-1">{label}</div>
            <div class="font-mono text-xs font-bold truncate tracking-wider">{value}</div>
        </div>
    }
}

#[component]
fn ChunkCard(chunk: ChunkPreview, strategy: ChunkStrategy, size_limit: u32) -> impl IntoView {
    let (show_tokens, set_show_tokens) = signal(false);

    let prefix = if chunk.is_glossary {
        "GLOSSARY"
    } else {
        "POST_BODY"
    };
    let text_length = chunk.text_length;
    let token_count = chunk.token_count;
    let heading = chunk.heading.clone();

    let (length_label, count_label, over_limit) = match strategy {
        ChunkStrategy::Bert => (
            format!("LENGTH: {text_length} CHARS"),
            format!("TOKENS: {token_count}/{size_limit}"),
            token_count > size_limit,
        ),
        ChunkStrategy::Section => (
            format!("LENGTH: {text_length}/{size_limit} CHARS"),
            format!("TOKENS: {token_count}"),
            text_length > size_limit,
        ),
    };

    let tokens = StoredValue::new(chunk.tokens);
    let text_excerpt = StoredValue::new(chunk.text_excerpt);

    let count_class = if over_limit {
        "log-line-error font-bold"
    } else {
        "opacity-40"
    };

    view! {
        <div class="card-inner p-3 relative overflow-hidden group">
            <div class="flex flex-row justify-between">
            <div class="flex flex-col mb-2">
                <span class="tech-label text-[var(--color-accent)]">{prefix}</span>
                <span class="font-bold text-sm uppercase tracking-tight">{heading}</span>
            </div>
            <div class="flex gap-1 mb-2 py-2 justify-end">
                <button
                    type="button"
                    class=move || tab_class(!show_tokens.get())
                    on:click=move |_| set_show_tokens.set(false)
                >
                    "TEXT"
                </button>
                <button
                    type="button"
                    class=move || tab_class(show_tokens.get())
                    on:click=move |_| set_show_tokens.set(true)
                >
                    "TOKENS"
                </button>
                </div>
            </div>
            {move || {
                if show_tokens.get() {
                    view! {
                        <div class="log-pre text-[10px] bg-transparent border-none p-0 flex flex-wrap gap-1 max-h-[14rem] overflow-auto">
                            {tokens
                                .with_value(|toks| {
                                    toks.iter()
                                        .enumerate()
                                        .map(|(i, t)| {
                                            view! {
                                                <span
                                                    class="token-pill"
                                                    title=i.to_string()
                                                >
                                                    {t.clone()}
                                                </span>
                                            }
                                        })
                                        .collect_view()
                                })}
                        </div>
                    }
                        .into_any()
                } else {
                    view! {
                        <pre class="log-pre text-[10px] bg-transparent border-none p-0 max-h-[10rem]">
                            {text_excerpt.get_value()}
                        </pre>
                    }
                        .into_any()
                }
            }}
            <div class="mt-2 flex justify-between items-center tech-label">
                <span class="opacity-40">{length_label}</span>
                <span class=count_class>{count_label}</span>
            </div>
            <div class="mt-2 pt-2 border-t border-[var(--color-border)] flex justify-end">
                <a
                    href=text_excerpt.with_value(|t| format!("/embed?a={}", urlencoding::encode(t)))
                    class="tech-label opacity-40 hover:opacity-100 transition-opacity"
                >
                    "PROBE_EMBED →"
                </a>
            </div>
        </div>
    }
}

fn tab_class(active: bool) -> &'static str {
    if active {
        "tech-label px-2 py-0.5 border border-[var(--color-accent-strong)] bg-[var(--color-tag-bg)] cursor-pointer"
    } else {
        "tech-label opacity-50 px-2 py-0.5 border border-[var(--color-border)] cursor-pointer"
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
