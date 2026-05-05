use crate::shared::{ChunkStrategy, ChunkingConfig};
use leptos::prelude::*;

#[component]
pub fn TuningPanel(
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
        ChunkStrategy::Llm => "llm",
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
                    "Tune chunking for this post only. Save to apply the override to preview and ingest."
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
                                    "llm" => ChunkStrategy::Llm,
                                    _ => ChunkStrategy::Bert,
                                };
                            }));
                        }
                    >
                        <option value="bert">"bert"</option>
                        <option value="section">"section"</option>
                        <option value="llm">"llm"</option>
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
                    ChunkStrategy::Llm => view! {
                        <SmallField label="MICRO_CHUNK_CHARS">
                            <input
                                class="input font-mono text-xs"
                                type="number"
                                min="100"
                                prop:value=move || working.get().llm_micro_chunk_chars.to_string()
                                on:input=move |e| {
                                    let v: u32 = event_target_value(&e).parse().unwrap_or(300);
                                    update(Box::new(move |c| c.llm_micro_chunk_chars = v.max(100)));
                                }
                            />
                        </SmallField>
                    }.into_any()
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
                    <Show when=move || has_unsaved_changes()>
                        <button
                            type="button"
                            class="btn btn-primary"
                            on:click=save
                        >
                            "SAVE_OVERRIDE"
                        </button>
                    </Show>
                    <Show when=move || is_overridden()>
                        <button
                            type="button"
                            class="btn"
                            on:click=reset
                        >
                            "RESET"
                        </button>
                    </Show>
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
