use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;

use crate::server_functions::embed::embed_texts;
use crate::shared::EmbedResult;

#[component]
pub fn EmbedTestPage() -> impl IntoView {
    let query = use_query_map();
    let initial_a = query.with(|q| q.get("a").unwrap_or_default().to_string());
    let initial_b = query.with(|q| q.get("b").unwrap_or_default().to_string());

    let (model, set_model) = signal("qwen3-embedding:0.6b".to_string());
    let (text_a, set_text_a) = signal(initial_a);
    let (text_b, set_text_b) = signal(initial_b);
    let (result, set_result) = signal::<Option<EmbedResult>>(None);
    let (error, set_error) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(false);

    let compute = move |_| {
        let m = model.get_untracked();
        let a = text_a.get_untracked();
        let b = text_b.get_untracked();

        if a.trim().is_empty() || b.trim().is_empty() {
            set_error.set(Some("Both text fields are required.".to_string()));
            return;
        }

        set_loading.set(true);
        set_error.set(None);
        set_result.set(None);

        spawn_local(async move {
            match embed_texts(m, a, b).await {
                Ok(r) => {
                    set_result.set(Some(r));
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("EMBED_FAULT: {e}")));
                    set_loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="space-y-6 max-w-6xl">
            <div class="flex flex-col border-b border-[var(--color-border)] pb-4">
                <h1 class="text-2xl font-bold tracking-tight uppercase">"EMBED_TESTER"</h1>
                <p class="text-[10px] mt-2 font-mono opacity-50">
                    "COMPUTE_COSINE_SIMILARITY"
                </p>
            </div>

            <div class="card-outer p-4 flex flex-col sm:flex-row gap-4 items-center">
                <label class="block flex-1 space-y-1.5">
                    <div class="tech-label opacity-70">"MODEL_ID"</div>
                    <input
                        class="input font-mono text-sm"
                        prop:value=model
                        on:input=move |e| set_model.set(event_target_value(&e))
                    />
                    <div class="tech-label text-[9px] opacity-40">
                        "> MODEL_NAME (e.g. qwen3-embedding:0.6b, @cf/baai/bge-base-en-v1.5)"
                    </div>
                </label>
                <button
                    class="btn btn-primary whitespace-nowrap"
                    disabled=move || loading.get()
                    on:click=compute
                >
                    {move || if loading.get() { "COMPUTING..." } else { "COMPUTE_SIMILARITY" }}
                </button>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
                <div class="card-outer p-4 space-y-2">
                    <span class="tech-label">"input.text_a"</span>
                    <textarea
                        class="input font-mono text-sm h-48 resize-y"
                        prop:value=text_a
                        on:input=move |e| set_text_a.set(event_target_value(&e))
                    />
                </div>
                <div class="card-outer p-4 space-y-2">
                    <span class="tech-label">"input.text_b"</span>
                    <textarea
                        class="input font-mono text-sm h-48 resize-y"
                        prop:value=text_b
                        on:input=move |e| set_text_b.set(event_target_value(&e))
                    />
                </div>
            </div>

            {move || {
                error
                    .get()
                    .map(|e| {
                        view! {
                            <div class="card-outer p-4 log-line-error font-mono text-sm">{e}</div>
                        }
                    })
            }}

            {move || {
                result
                    .get()
                    .map(|r| {
                        let EmbedResult { dims, norm_a, norm_b, similarity } = r;
                        view! {
                            <div class="card-outer p-6 space-y-6">
                                <div class="flex flex-col">
                                    <span class="tech-label">"output.similarity"</span>
                                </div>

                                <div class="flex flex-col items-center text-center py-4 border-b border-[var(--color-border)]">
                                    <span class=format!(
                                        "text-6xl font-mono font-bold {}",
                                        similarity_class(similarity),
                                    )>{format!("{:.4}", similarity)}</span>
                                    <span class=format!(
                                        "tech-label mt-3 {}",
                                        similarity_class(similarity),
                                    )>{similarity_label(similarity)}</span>
                                    <p class="text-[10px] font-mono opacity-40 mt-3 max-w-md leading-relaxed">
                                        "Cosine similarity measures the angle between two vectors in embedding space. \
                                         1.0 = identical direction (same meaning), 0.0 = orthogonal (unrelated), −1.0 = opposite meaning."
                                    </p>
                                </div>

                                <div class="space-y-3">
                                    <div class="grid grid-cols-3 gap-0 border-x border-t border-[var(--color-border)]">
                                        <div class="p-3 border-r border-b border-[var(--color-border)] bg-[var(--color-card-inner)]/50">
                                            <div class="tech-label opacity-50 mb-1">"DIMENSIONS"</div>
                                            <div class="font-mono text-xs font-bold truncate tracking-wider">
                                                {dims.to_string()}
                                            </div>
                                        </div>
                                        <div class="p-3 border-r border-b border-[var(--color-border)] bg-[var(--color-card-inner)]/50">
                                            <div class="tech-label opacity-50 mb-1">"NORM_A"</div>
                                            <div class=format!(
                                                "font-mono text-xs font-bold truncate tracking-wider {}",
                                                norm_class(norm_a),
                                            )>{format!("{:.4}", norm_a)}</div>
                                        </div>
                                        <div class="p-3 border-r border-b border-[var(--color-border)] bg-[var(--color-card-inner)]/50">
                                            <div class="tech-label opacity-50 mb-1">"NORM_B"</div>
                                            <div class=format!(
                                                "font-mono text-xs font-bold truncate tracking-wider {}",
                                                norm_class(norm_b),
                                            )>{format!("{:.4}", norm_b)}</div>
                                        </div>
                                    </div>
                                    <p class="tech-label text-[9px] opacity-40">
                                        "NORM: L2 magnitude of the embedding vector. \
                                         Well-behaved models normalize to ~1.0. \
                                         Values far from 1.0 may indicate a model misconfiguration."
                                    </p>
                                </div>
                            </div>
                        }
                    })
            }}
        </div>
    }
}

fn similarity_class(s: f32) -> &'static str {
    if s >= 0.85 {
        "log-line-success"
    } else if s >= 0.65 {
        "text-amber-400"
    } else if s >= 0.45 {
        ""
    } else {
        "log-line-error"
    }
}

fn similarity_label(s: f32) -> &'static str {
    if s >= 0.85 {
        "HIGHLY_SIMILAR"
    } else if s >= 0.65 {
        "RELATED"
    } else if s >= 0.45 {
        "LOOSELY_RELATED"
    } else {
        "UNRELATED"
    }
}

fn norm_class(n: f32) -> &'static str {
    if (n - 1.0).abs() < 0.1 {
        "log-line-success"
    } else {
        "text-amber-400"
    }
}
