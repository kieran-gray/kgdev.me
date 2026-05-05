use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::server_fns::{load_settings, save_settings};
use crate::shared::{
    catalog_for_backend, CatalogEntry, ChunkStrategy, ChunkingConfig, EmbedderBackend,
    EmbeddingModel, EvaluationGenerationBackend, EvaluationSettings, SettingsDto,
    VectorIndexConfig,
};

#[component]
pub fn SettingsPage() -> impl IntoView {
    let initial = Resource::new(|| (), |_| async move { load_settings().await });

    view! {
        <div class="space-y-6 max-w-6xl">
            <div class="flex flex-col border-b border-[var(--color-border)] pb-4">
                <h1 class="text-2xl font-bold tracking-tight uppercase">"CONFIGURATION_PANEL"</h1>
                <p class="text-[10px] mt-2 font-mono opacity-50">
                    "LOCAL_STORAGE_REF: ./rag-admin/data/settings.toml"
                </p>
            </div>
            <Suspense fallback=|| view! { <p class="tech-label animate-pulse">"FETCHING_CONFIG..."</p> }>
                {move || {
                    initial
                        .get()
                        .map(|res| match res {
                            Ok(s) => view! { <SettingsForm initial=s /> }.into_any(),
                            Err(e) => {
                                view! {
                                    <div class="card-outer p-4 log-line-error font-mono text-sm">
                                        {format!("CONFIG_LOAD_FAULT: {e}")}
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
fn SettingsForm(initial: SettingsDto) -> impl IntoView {
    let (blog_url, set_blog_url) = signal(initial.blog_url);
    let (account, set_account) = signal(initial.cloudflare_account_id);
    let (token, set_token) = signal(initial.cloudflare_api_token);
    let (kv_ns, set_kv_ns) = signal(initial.kv_namespace_id);
    let (vector_index, set_vector_index) = signal(initial.vector_index);
    let (model, set_model) = signal(initial.embedding_model);
    let (default_chunking, set_default_chunking) = signal(initial.default_chunking);
    let (evaluation, set_evaluation) = signal(initial.evaluation);

    let (status, set_status) = signal::<Option<(bool, String)>>(None);

    let on_save = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        set_status.set(None);
        let payload = SettingsDto {
            blog_url: blog_url.get(),
            cloudflare_account_id: account.get(),
            cloudflare_api_token: token.get(),
            kv_namespace_id: kv_ns.get(),
            vector_index: vector_index.get(),
            embedding_model: model.get(),
            default_chunking: default_chunking.get(),
            evaluation: evaluation.get(),
        };
        spawn_local(async move {
            match save_settings(payload).await {
                Ok(()) => set_status.set(Some((true, "STATE_SYNC_COMPLETE".to_string()))),
                Err(e) => set_status.set(Some((false, format!("SYNC_FAULT: {e}")))),
            }
        });
    };

    view! {
        <form on:submit=on_save class="space-y-6">
            <VectorIndexCard
                vector_index=vector_index
                set_vector_index=set_vector_index
            />

            <ModelCard
                vector_index=vector_index
                model=model
                set_model=set_model
            />

            <ChunkingDefaultsSection
                config=default_chunking
                set_config=set_default_chunking
            />

            <EvaluationDefaultsSection
                config=evaluation
                set_config=set_evaluation
            />

            <div class="card-outer p-6 space-y-4">
                <div class="flex flex-col border-b border-[var(--color-border)] pb-3">
                    <span class="tech-label">"cloudflare.config"</span>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <Field label="KV_NAMESPACE_ID" hint="STORAGE_CACHE_ID">
                        <input
                            class="input font-mono text-sm"
                            prop:value=kv_ns
                            on:input=move |e| set_kv_ns.set(event_target_value(&e))
                        />
                    </Field>
                    <Field label="CF_ACCOUNT_ID" hint="CLOUDFLARE_AUTH_CONTEXT">
                        <input
                            class="input font-mono text-sm"
                            prop:value=account
                            on:input=move |e| set_account.set(event_target_value(&e))
                        />
                    </Field>
                    <Field label="CF_API_TOKEN" hint="ENCRYPTED_AUTH_TOKEN">
                        <input
                            class="input font-mono text-sm"
                            type="password"
                            prop:value=token
                            on:input=move |e| set_token.set(event_target_value(&e))
                        />
                    </Field>
                </div>
            </div>

            <div class="card-outer p-6 space-y-4">
                <div class="flex flex-col border-b border-[var(--color-border)] pb-3">
                    <span class="tech-label">"blog.config"</span>
                </div>
                <div class="grid grid-cols-1 gap-6">
                    <Field label="BLOG_BASE_URL" hint="TARGET_URI (e.g. https://kgdev.me)">
                        <input
                            class="input font-mono text-sm"
                            prop:value=blog_url
                            on:input=move |e| set_blog_url.set(event_target_value(&e))
                        />
                    </Field>
                </div>
            </div>

            <div class="flex items-center gap-4">
                <button class="btn btn-primary px-8" type="submit">"SAVE_CHANGES"</button>
                {move || {
                    status
                        .get()
                        .map(|(ok, msg)| {
                            let cls = if ok { "text-emerald-500" } else { "text-red-500" };
                            view! { <span class=format!("tech-label {}", cls)>{msg}</span> }
                        })
                }}
            </div>
        </form>
    }
}

#[component]
fn EvaluationDefaultsSection(
    config: ReadSignal<EvaluationSettings>,
    set_config: WriteSignal<EvaluationSettings>,
) -> impl IntoView {
    view! {
        <div class="card-outer p-6 space-y-4">
            <div class="flex flex-col border-b border-[var(--color-border)] pb-3">
                <span class="tech-label">"chunking.evaluation"</span>
                <p class="tech-label opacity-50 mt-2">
                    "Defaults for synthetic chunking evaluation. Generation is Ollama-first so local runs do not spend Workers AI credits."
                </p>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
                <Field label="GENERATION_BACKEND" hint="ollama is the supported local default">
                    <select
                        class="input font-mono text-sm"
                        prop:value=move || config.get().generation_backend.as_str().to_string()
                        on:change=move |e| {
                            let v = event_target_value(&e);
                            set_config.update(|c| {
                                c.generation_backend = match v.as_str() {
                                    "workers_ai" => EvaluationGenerationBackend::WorkersAi,
                                    _ => EvaluationGenerationBackend::Ollama,
                                };
                            });
                        }
                    >
                        <option value="ollama">"ollama"</option>
                        <option value="workers_ai" disabled=true>"workers_ai (later)"</option>
                    </select>
                </Field>
                <Field label="OLLAMA_BASE_URL" hint="local Ollama daemon URL">
                    <input
                        class="input font-mono text-sm"
                        prop:value=move || config.get().ollama_base_url
                        on:input=move |e| {
                            let v = event_target_value(&e);
                            set_config.update(|c| c.ollama_base_url = v);
                        }
                    />
                </Field>
                <Field label="GENERATION_MODEL" hint="chat model used to create questions">
                    <input
                        class="input font-mono text-sm"
                        prop:value=move || config.get().generation_model
                        on:input=move |e| {
                            let v = event_target_value(&e);
                            set_config.update(|c| c.generation_model = v);
                        }
                    />
                </Field>
                <Field label="QUESTION_COUNT" hint="target generated questions per post">
                    <input
                        class="input font-mono text-sm"
                        type="number"
                        min="1"
                        prop:value=move || config.get().question_count.to_string()
                        on:input=move |e| {
                            let v: u32 = event_target_value(&e).parse().unwrap_or(0);
                            set_config.update(|c| c.question_count = v);
                        }
                    />
                </Field>
                <Field label="TOP_K" hint="chunks retrieved per synthetic question">
                    <input
                        class="input font-mono text-sm"
                        type="number"
                        min="1"
                        prop:value=move || config.get().top_k.to_string()
                        on:input=move |e| {
                            let v: u32 = event_target_value(&e).parse().unwrap_or(0);
                            set_config.update(|c| c.top_k = v);
                        }
                    />
                </Field>
                <Field label="MIN_SCORE_MILLI" hint="0-1000 cosine threshold for retrieved chunks">
                    <input
                        class="input font-mono text-sm"
                        type="number"
                        min="0"
                        max="1000"
                        prop:value=move || config.get().min_score_milli.to_string()
                        on:input=move |e| {
                            let v: u32 = event_target_value(&e).parse().unwrap_or(0);
                            set_config.update(|c| c.min_score_milli = v);
                        }
                    />
                </Field>
                <Field label="EXCERPT_THRESHOLD" hint="0-1000; filters weak query/reference pairs">
                    <input
                        class="input font-mono text-sm"
                        type="number"
                        min="0"
                        max="1000"
                        prop:value=move || config.get().excerpt_similarity_threshold_milli.to_string()
                        on:input=move |e| {
                            let v: u32 = event_target_value(&e).parse().unwrap_or(0);
                            set_config.update(|c| c.excerpt_similarity_threshold_milli = v);
                        }
                    />
                </Field>
                <Field label="DUPLICATE_THRESHOLD" hint="0-1000; filters near-duplicate questions">
                    <input
                        class="input font-mono text-sm"
                        type="number"
                        min="0"
                        max="1000"
                        prop:value=move || config.get().duplicate_similarity_threshold_milli.to_string()
                        on:input=move |e| {
                            let v: u32 = event_target_value(&e).parse().unwrap_or(0);
                            set_config.update(|c| c.duplicate_similarity_threshold_milli = v);
                        }
                    />
                </Field>
                <Field label="INCLUDE_GLOSSARY" hint="include glossary chunks as retrieval distractors">
                    <select
                        class="input font-mono text-sm"
                        prop:value=move || if config.get().include_glossary { "true" } else { "false" }
                        on:change=move |e| {
                            let v = event_target_value(&e);
                            set_config.update(|c| c.include_glossary = v == "true");
                        }
                    >
                        <option value="true">"true"</option>
                        <option value="false">"false"</option>
                    </select>
                </Field>
            </div>
        </div>
    }
}

#[component]
fn VectorIndexCard(
    vector_index: ReadSignal<VectorIndexConfig>,
    set_vector_index: WriteSignal<VectorIndexConfig>,
) -> impl IntoView {
    let provider_str = move || vector_index.get().provider().as_str().to_string();
    let name_value = move || vector_index.get().name().to_string();
    let dims_value = move || vector_index.get().dimensions();

    let on_name_input = move |e: leptos::ev::Event| {
        let v = event_target_value(&e);
        set_vector_index.update(|vi| *vi = vi.clone().with_name(v));
    };

    let on_dims_input = move |e: leptos::ev::Event| {
        let v: u32 = event_target_value(&e).parse().unwrap_or(0);
        set_vector_index.update(|vi| *vi = vi.clone().with_dimensions(v));
    };

    view! {
        <div class="card-outer p-6 space-y-4">
            <div class="flex flex-col border-b border-[var(--color-border)] pb-3">
                <span class="tech-label">"vector_index"</span>
                <p class="tech-label opacity-50 mt-2">
                    "Where ingested vectors are written and where the Q&A backend reads from. \
                     The index dimensions are immutable on the provider."
                </p>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
                <Field label="PROVIDER" hint="local provider TODO">
                    <select
                        class="input font-mono text-sm"
                        prop:value=provider_str
                    >
                        <option value="cloudflare">"cloudflare"</option>
                    </select>
                </Field>
                <Field label="INDEX_NAME" hint="Cloudflare Vectorize index id">
                    <input
                        class="input font-mono text-sm"
                        prop:value=name_value
                        on:input=on_name_input
                    />
                </Field>
                <Field label="DIMENSIONS" hint="must match the index — click VERIFY to read the live value">
                    <input
                        class="input font-mono text-sm"
                        type="number"
                        min="1"
                        prop:value=move || dims_value().to_string()
                        on:input=on_dims_input
                    />
                </Field>
            </div>
        </div>
    }
}

#[component]
fn ModelCard(
    vector_index: ReadSignal<VectorIndexConfig>,
    model: ReadSignal<EmbeddingModel>,
    set_model: WriteSignal<EmbeddingModel>,
) -> impl IntoView {
    let target_dims = move || vector_index.get().dimensions();
    let backend = move || model.get().backend;
    let catalog = move || catalog_for_backend(backend());

    let suggested_models = move || -> Vec<CatalogEntry> {
        let dims = target_dims();
        match backend() {
            EmbedderBackend::Cloudflare => catalog()
                .iter()
                .filter(|e| e.dims == dims)
                .copied()
                .collect(),
            EmbedderBackend::Ollama => catalog().to_vec(),
        }
    };

    let cloudflare_incompatible = move || -> Vec<CatalogEntry> {
        let dims = target_dims();
        match backend() {
            EmbedderBackend::Cloudflare => catalog()
                .iter()
                .filter(|e| e.dims != dims)
                .copied()
                .collect(),
            EmbedderBackend::Ollama => Vec::new(),
        }
    };

    let current_id = move || model.get().id;
    let current_dims = move || model.get().dims;

    let in_catalog = move || {
        let id = current_id();
        catalog().iter().any(|e| e.id == id)
    };

    let dims_locked = move || matches!(backend(), EmbedderBackend::Cloudflare) && in_catalog();

    let select_value = move || {
        if in_catalog() {
            current_id()
        } else {
            "__custom__".into()
        }
    };

    let on_backend_change = move |e: leptos::ev::Event| {
        let v = event_target_value(&e);
        let next_backend = match v.as_str() {
            "ollama" => EmbedderBackend::Ollama,
            _ => EmbedderBackend::Cloudflare,
        };

        let dims = target_dims();
        let new_catalog = catalog_for_backend(next_backend);
        let suggested = match next_backend {
            EmbedderBackend::Cloudflare => new_catalog.iter().find(|e| e.dims == dims).copied(),
            EmbedderBackend::Ollama => new_catalog.first().copied(),
        };
        match suggested {
            Some(entry) => set_model.set(EmbeddingModel {
                backend: next_backend,
                id: entry.id.into(),
                dims: if matches!(next_backend, EmbedderBackend::Ollama) {
                    dims
                } else {
                    entry.dims
                },
            }),
            None => set_model.update(|m| {
                m.backend = next_backend;
                m.id = String::new();
            }),
        }
    };

    let on_select = move |e: leptos::ev::Event| {
        let v = event_target_value(&e);
        if v == "__custom__" {
            set_model.update(|m| {
                if catalog().iter().any(|e| e.id == m.id) {
                    m.id = String::new();
                }
            });
        } else if let Some(entry) = catalog().iter().find(|e| e.id == v).copied() {
            set_model.update(|m| {
                m.id = entry.id.into();
                if matches!(m.backend, EmbedderBackend::Cloudflare) {
                    m.dims = entry.dims;
                }
            });
        }
    };

    let on_dims_input = move |e: leptos::ev::Event| {
        let v: u32 = event_target_value(&e).parse().unwrap_or(0);
        set_model.update(|m| m.dims = v);
    };

    view! {
        <div class="card-outer p-6 space-y-4">
            <div class="flex flex-col border-b border-[var(--color-border)] pb-3">
                <span class="tech-label">"embedding_model"</span>
                <p class="tech-label opacity-50 mt-2">
                    "Pick a backend (Cloudflare Workers AI or local Ollama) and a model. \
                     The vector index dimensions and the model output dimensions must match. \
                     Note: production Q&A expects ingest vectors to come from the same model the Q&A worker uses."
                </p>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
                <Field label="BACKEND" hint="cloudflare = Workers AI, ollama = local daemon">
                    <select
                        class="input font-mono text-sm"
                        prop:value=move || backend().as_str().to_string()
                        on:change=on_backend_change
                    >
                        <option value="cloudflare">"cloudflare"</option>
                        <option value="ollama">"ollama"</option>
                    </select>
                </Field>
                <Field label="MODEL" hint="cloudflare: filtered to dim-compatible · ollama: suggestions only">
                    <select
                        class="input font-mono text-sm"
                        prop:value=select_value
                        on:change=on_select
                    >
                        {move || {
                            suggested_models()
                                .into_iter()
                                .map(|e| view! { <option value=e.id>{format!("{} ({} dims)", e.id, e.dims)}</option> })
                                .collect_view()
                        }}
                        {move || {
                            let inc = cloudflare_incompatible();
                            if inc.is_empty() {
                                ().into_any()
                            } else {
                                view! {
                                    <optgroup label="incompatible (dims mismatch)">
                                        {inc
                                            .into_iter()
                                            .map(|e| view! {
                                                <option value=e.id disabled=true>
                                                    {format!("{} ({} dims)", e.id, e.dims)}
                                                </option>
                                            })
                                            .collect_view()}
                                    </optgroup>
                                }.into_any()
                            }
                        }}
                    </select>
                </Field>
                <Field label="OUTPUT_DIMS" hint="must match the vector index dimensions">
                    <input
                        class=move || {
                            if dims_locked() {
                                "input font-mono text-sm opacity-60"
                            } else {
                                "input font-mono text-sm"
                            }
                        }
                        type="number"
                        min="1"
                        prop:value=move || current_dims().to_string()
                        disabled=dims_locked
                        on:input=on_dims_input
                    />
                </Field>
            </div>
            {move || {
                if matches!(backend(), EmbedderBackend::Cloudflare) && suggested_models().is_empty() {
                    view! {
                        <div class="tech-label log-line-error">
                            {format!(
                                "NO COMPATIBLE CLOUDFLARE MODELS at {} dims — adjust index dimensions or switch backend.",
                                target_dims()
                            )}
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }
            }}
        </div>
    }
}

#[component]
fn ChunkingDefaultsSection(
    config: ReadSignal<ChunkingConfig>,
    set_config: WriteSignal<ChunkingConfig>,
) -> impl IntoView {
    view! {
        <div class="card-outer p-6 space-y-4">
            <div class="flex flex-col border-b border-[var(--color-border)] pb-3">
                <span class="tech-label">"chunking.defaults"</span>
                <p class="tech-label opacity-50 mt-2">
                    "Default chunking parameters for ingest. \
                     Per-post overrides can be previewed and saved from the post detail page."
                </p>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
                <Field label="STRATEGY" hint="bert = sliding window w/ overlap, section = one chunk per H2/H3, llm = semantic boundaries over micro chunks">
                    <select
                        class="input font-mono text-sm"
                        prop:value=move || match config.get().strategy {
                            ChunkStrategy::Bert => "bert",
                            ChunkStrategy::Section => "section",
                            ChunkStrategy::Llm => "llm",
                        }
                        on:change=move |e| {
                            let v = event_target_value(&e);
                            set_config.update(|c| {
                                c.strategy = match v.as_str() {
                                    "section" => ChunkStrategy::Section,
                                    "llm" => ChunkStrategy::Llm,
                                    _ => ChunkStrategy::Bert,
                                };
                            });
                        }
                    >
                        <option value="bert">"bert"</option>
                        <option value="section">"section"</option>
                        <option value="llm">"llm"</option>
                    </select>
                </Field>
                <Field label="MAX_SECTION_CHARS" hint="section: max chars per chunk before fallback split">
                    <input
                        class="input font-mono text-sm"
                        type="number"
                        min="1"
                        prop:value=move || config.get().max_section_chars.to_string()
                        on:input=move |e| {
                            let v: u32 = event_target_value(&e).parse().unwrap_or(0);
                            set_config.update(|c| c.max_section_chars = v);
                        }
                    />
                </Field>
                <Field label="TARGET_CHARS" hint="bert: target chunk size in chars">
                    <input
                        class="input font-mono text-sm"
                        type="number"
                        min="1"
                        prop:value=move || config.get().target_chars.to_string()
                        on:input=move |e| {
                            let v: u32 = event_target_value(&e).parse().unwrap_or(0);
                            set_config.update(|c| c.target_chars = v);
                        }
                    />
                </Field>
                <Field label="OVERLAP_CHARS" hint="bert: chars of overlap between adjacent chunks">
                    <input
                        class="input font-mono text-sm"
                        type="number"
                        min="0"
                        prop:value=move || config.get().overlap_chars.to_string()
                        on:input=move |e| {
                            let v: u32 = event_target_value(&e).parse().unwrap_or(0);
                            set_config.update(|c| c.overlap_chars = v);
                        }
                    />
                </Field>
                <Field label="MIN_CHARS" hint="bert: small trailing chunks merge with the previous one">
                    <input
                        class="input font-mono text-sm"
                        type="number"
                        min="0"
                        prop:value=move || config.get().min_chars.to_string()
                        on:input=move |e| {
                            let v: u32 = event_target_value(&e).parse().unwrap_or(0);
                            set_config.update(|c| c.min_chars = v);
                        }
                    />
                </Field>
                <Field label="LLM_MICRO_CHUNK_CHARS" hint="llm: punctuation-aware micro chunks offered to the model for boundary selection">
                    <input
                        class="input font-mono text-sm"
                        type="number"
                        min="100"
                        prop:value=move || config.get().llm_micro_chunk_chars.to_string()
                        on:input=move |e| {
                            let v: u32 = event_target_value(&e).parse().unwrap_or(300);
                            set_config.update(|c| c.llm_micro_chunk_chars = v.max(100));
                        }
                    />
                </Field>
            </div>
        </div>
    }
}

#[component]
fn Field(label: &'static str, hint: &'static str, children: Children) -> impl IntoView {
    view! {
        <label class="block space-y-1.5">
            <div class="tech-label opacity-70">
                {label}
            </div>
            {children()}
            <div class="tech-label text-[9px] opacity-40">
                {format!("> {}", hint)}
            </div>
        </label>
    }
}
