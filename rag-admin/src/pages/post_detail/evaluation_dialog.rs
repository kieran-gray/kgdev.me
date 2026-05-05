use crate::components::log_panel::LogPanel;
use crate::pages::post_detail::utils::{
    open_event_stream, open_event_stream_with_done, short_hash, sweep_variants,
};
use crate::server_fns::{
    get_evaluation_dataset_status, get_latest_evaluation_result, start_generate_evaluation_dataset,
    start_run_evaluation,
};
use crate::shared::{
    ChunkingConfig, ChunkingVariant, EvaluationDatasetStatus, EvaluationRunOptions,
    EvaluationRunResult, LogEvent, LogLevel,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn EvaluationDialog(
    slug: String,
    current_config: ChunkingConfig,
    open: ReadSignal<bool>,
    set_open: WriteSignal<bool>,
    set_eval_result: WriteSignal<Option<Result<EvaluationRunResult, String>>>,
) -> impl IntoView {
    let slug_value = StoredValue::new(slug);
    let current_config_stored = StoredValue::new(current_config);
    let (refresh, set_refresh) = signal(0u32);
    let (events, set_events) = signal::<Vec<LogEvent>>(Vec::new());
    let (running, set_running) = signal(false);
    let (eval_running, set_eval_running) = signal(false);

    let (top_k, set_top_k) = signal(5u32);
    let (min_score_milli, set_min_score_milli) = signal(0u32);
    let (include_glossary, set_include_glossary) = signal(true);

    let status = Resource::new(
        move || (slug_value.get_value(), refresh.get()),
        move |(slug, _)| async move {
            get_evaluation_dataset_status(slug)
                .await
                .map_err(|e| e.to_string())
        },
    );

    let generate = move |_| {
        if running.get_untracked() {
            return;
        }
        set_running.set(true);
        set_events.set(vec![LogEvent {
            level: LogLevel::Info,
            message: "INIT_PROCESS: generate chunking evaluation dataset...".into(),
        }]);
        let slug = slug_value.get_value();
        spawn_local(async move {
            match start_generate_evaluation_dataset(slug).await {
                Ok(info) => open_event_stream(info.stream_url, set_events, set_running),
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

    let run_eval = move |variants: Vec<ChunkingVariant>| {
        if eval_running.get_untracked() {
            return;
        }
        set_eval_running.set(true);
        set_eval_result.set(None);
        set_events.set(Vec::new());

        let slug = slug_value.get_value();
        let options = EvaluationRunOptions {
            top_k: top_k.get_untracked(),
            min_score_milli: min_score_milli.get_untracked(),
            include_glossary: include_glossary.get_untracked(),
        };

        let slug_for_result = slug.clone();
        let expected_variants = variants.clone();
        let expected_options = options.clone();

        spawn_local(async move {
            match start_run_evaluation(slug, variants, options).await {
                Ok(info) => {
                    open_event_stream_with_done(
                        info.stream_url,
                        set_events,
                        set_eval_running,
                        move || {
                            let slug = slug_for_result.clone();
                            let variants = expected_variants.clone();
                            let options = expected_options.clone();
                            spawn_local(async move {
                                let result = get_latest_evaluation_result(slug)
                                    .await
                                    .map_err(|e| e.to_string())
                                    .and_then(|saved| {
                                        saved.ok_or_else(|| {
                                            "evaluation finished without saving a result".to_string()
                                        })
                                    })
                                    .and_then(|saved| {
                                        if saved.options == options
                                            && saved
                                                .variants
                                                .iter()
                                                .map(|v| v.variant.clone())
                                                .collect::<Vec<_>>()
                                                == variants
                                        {
                                            Ok(saved)
                                        } else {
                                            Err("latest saved evaluation does not match the completed run".to_string())
                                        }
                                    });
                                set_eval_result.set(Some(result));
                            });
                        },
                    );
                }
                Err(e) => {
                    set_events.update(|evs| {
                        evs.push(LogEvent {
                            level: LogLevel::Error,
                            message: format!("PROCESS_FAILURE: {e}"),
                        })
                    });
                    set_eval_running.set(false);
                }
            }
        });
    };

    let run_current = move |_| {
        run_eval(vec![ChunkingVariant {
            label: "current".into(),
            config: current_config_stored.get_value(),
        }]);
    };

    let run_sweep = move |_| {
        run_eval(sweep_variants(current_config_stored.get_value()));
    };

    let close_dialog = move |_| {
        if running.get() || eval_running.get() {
            return;
        }
        set_open.set(false);
    };

    view! {
        <Show when=move || open.get()>
            <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
                <div class="card-outer p-6 w-full max-w-5xl mx-4 flex flex-col gap-4 max-h-[90vh]">
                    <div class="flex items-start justify-between border-b border-[var(--color-border)] pb-2">
                        <div class="flex flex-col">
                            <span class="tech-label">"process.evaluation"</span>
                            <h2 class="text-lg font-bold">"CHUNKING_EVALUATION_LAB"</h2>
                        </div>
                        <button
                            class="tech-label opacity-50 hover:opacity-100 px-2 py-0.5 border border-[var(--color-border)] cursor-pointer disabled:cursor-not-allowed disabled:opacity-20"
                            disabled=move || running.get() || eval_running.get()
                            on:click=close_dialog
                        >
                            "✕"
                        </button>
                    </div>

                    <div class="grid grid-cols-1 md:grid-cols-2 gap-6 overflow-auto py-2">
                        <div class="space-y-4">
                            <div class="space-y-2">
                                <span class="tech-label opacity-60">"DATASET_CONFIGURATION"</span>
                                <Suspense fallback=|| view! { <p class="tech-label animate-pulse">"CHECKING_DATASET..."</p> }>
                                    {move || {
                                        status
                                            .get()
                                            .map(|res| match res {
                                                Ok(status) => view! { <EvaluationStatusView status=status /> }.into_any(),
                                                Err(e) => view! {
                                                    <div class="tech-label log-line-error">
                                                        {format!("DATASET_STATUS_FAULT: {e}")}
                                                    </div>
                                                }.into_any(),
                                            })
                                    }}
                                </Suspense>
                                <div class="flex gap-2">
                                    <button
                                        type="button"
                                        class="btn btn-primary flex-1 justify-center"
                                        disabled=move || running.get()
                                        on:click=generate
                                    >
                                        {move || if running.get() { "GENERATING..." } else { "GENERATE_DATASET" }}
                                    </button>
                                    <button
                                        type="button"
                                        class="btn justify-center"
                                        disabled=move || running.get()
                                        on:click=move |_| set_refresh.update(|v| *v += 1)
                                    >
                                        "↻"
                                    </button>
                                </div>
                            </div>

                            <div class="space-y-2 pt-4 border-t border-[var(--color-border)]">
                                <span class="tech-label opacity-60">"EVALUATION_PARAMETERS"</span>
                                <div class="grid grid-cols-1 gap-3">
                                    <SmallField label="TOP_K">
                                        <input
                                            class="input font-mono text-xs"
                                            type="number"
                                            min="1"
                                            prop:value=move || top_k.get().to_string()
                                            on:input=move |e| {
                                                let v: u32 = event_target_value(&e).parse().unwrap_or(1);
                                                set_top_k.set(v.max(1));
                                            }
                                        />
                                    </SmallField>
                                    <SmallField label="MIN_SCORE_MILLI">
                                        <input
                                            class="input font-mono text-xs"
                                            type="number"
                                            min="0"
                                            max="1000"
                                            prop:value=move || min_score_milli.get().to_string()
                                            on:input=move |e| {
                                                let v: u32 = event_target_value(&e).parse().unwrap_or(0);
                                                set_min_score_milli.set(v.min(1000));
                                            }
                                        />
                                    </SmallField>
                                    <SmallField label="INCLUDE_GLOSSARY">
                                        <select
                                            class="input font-mono text-xs"
                                            prop:value=move || if include_glossary.get() { "true" } else { "false" }
                                            on:change=move |e| {
                                                set_include_glossary.set(event_target_value(&e) == "true");
                                            }
                                        >
                                            <option value="true">"true"</option>
                                            <option value="false">"false"</option>
                                        </select>
                                    </SmallField>
                                </div>
                            </div>
                        </div>

                        <div class="space-y-4 flex flex-col min-h-0 h-full">
                            <span class="tech-label opacity-60">"EXECUTION_LOG"</span>
                            <div class="flex-1 bg-black/20 min-h-0 flex flex-col border border-[var(--color-border)]">
                                <div class="flex-1 min-h-0 overflow-hidden">
                                    <LogPanel events=events />
                                </div>
                                {move || eval_running.get().then(|| view! {
                                    <div class="shrink-0 p-2 tech-label animate-pulse text-emerald-500 border-t border-[var(--color-border)] bg-black/30">
                                        "RUNNING_EVALUATION..."
                                    </div>
                                })}
                            </div>

                            <div class="grid grid-cols-2 gap-2 mt-auto px-2">
                                <button
                                    type="button"
                                    class="btn w-full justify-center"
                                    disabled=move || eval_running.get() || running.get()
                                    on:click=run_current
                                >
                                    "RUN_CURRENT"
                                </button>
                                <button
                                    type="button"
                                    class="btn btn-primary w-full justify-center"
                                    disabled=move || eval_running.get() || running.get()
                                    on:click=run_sweep
                                >
                                    "RUN_SWEEP"
                                </button>
                            </div>
                        </div>
                    </div>

                    <div class="flex justify-end pt-4 border-t border-[var(--color-border)]">
                        <button
                            class="btn px-8"
                            disabled=move || running.get() || eval_running.get()
                            on:click=close_dialog
                        >
                            "DONE"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}

#[component]
fn EvaluationStatusView(status: EvaluationDatasetStatus) -> impl IntoView {
    let label = if status.exists {
        format!("READY · {} QUESTION(S)", status.question_count)
    } else {
        "NOT_GENERATED".into()
    };
    let generated_at = status.generated_at.unwrap_or_else(|| "N/A".into());

    view! {
        <div class="grid grid-cols-1 gap-2">
            <div class="card-inner p-2">
                <div class="tech-label opacity-50">"DATASET_STATUS"</div>
                <div class="font-mono text-[10px] font-bold">{label}</div>
            </div>
            <div class="card-inner p-2">
                <div class="tech-label opacity-50">"POST_VERSION"</div>
                <div class="font-mono text-[10px] font-bold truncate">{short_hash(&status.post_version)}</div>
            </div>
            <div class="card-inner p-2">
                <div class="tech-label opacity-50">"GENERATED_AT"</div>
                <div class="font-mono text-[10px] font-bold truncate">{generated_at}</div>
            </div>
        </div>
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
