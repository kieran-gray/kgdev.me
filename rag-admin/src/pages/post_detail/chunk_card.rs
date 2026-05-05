use crate::shared::{ChunkPreview, ChunkStrategy};
use leptos::prelude::*;

#[component]
pub fn ChunkCard(chunk: ChunkPreview, strategy: ChunkStrategy, size_limit: u32) -> impl IntoView {
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
        ChunkStrategy::Bert | ChunkStrategy::Llm => (
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
