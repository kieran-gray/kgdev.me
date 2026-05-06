use crate::components::log_panel::LogPanel;
use crate::pages::post_detail::utils::{
    chunking_variant_label, open_event_stream, open_event_stream_with_done, short_hash,
    sweep_variants,
};
use crate::server_functions::evaluation::{
    get_evaluation_dataset_status, get_latest_evaluation_result, start_generate_evaluation_dataset,
    start_run_evaluation, start_run_evaluation_autotune, start_run_evaluation_matrix,
};
use crate::shared::{
    ChunkingConfig, ChunkingVariant, EvaluationAutotuneRequest, EvaluationDatasetStatus,
    EvaluationRunOptions, EvaluationRunResult, LogEvent, LogLevel,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvaluationLabTab {
    Dataset,
    Sweep,
    Matrix,
    Autotune,
}

#[component]
pub fn EvaluationDialog(
    slug: String,
    current_config: ChunkingConfig,
    open: ReadSignal<bool>,
    set_open: WriteSignal<bool>,
    set_eval_result: WriteSignal<Option<Result<EvaluationRunResult, String>>>,
    set_history_refresh: WriteSignal<u32>,
) -> impl IntoView {
    let slug_value = StoredValue::new(slug);
    let current_config_stored = StoredValue::new(current_config);
    let matrix_variants = StoredValue::new(sweep_variants(current_config));
    let (refresh, set_refresh) = signal(0u32);
    let (events, set_events) = signal::<Vec<LogEvent>>(Vec::new());
    let (running, set_running) = signal(false);
    let (eval_running, set_eval_running) = signal(false);
    let (active_tab, set_active_tab) = signal(EvaluationLabTab::Dataset);

    let (top_k, set_top_k) = signal(5u32);
    let (min_score_milli, set_min_score_milli) = signal(0u32);
    let (include_glossary, set_include_glossary) = signal(true);
    let (matrix_variant_label, set_matrix_variant_label) =
        signal(chunking_variant_label(&current_config));
    let (matrix_top_k_values, set_matrix_top_k_values) = signal("2,3,5,8".to_string());
    let (matrix_min_score_values, set_matrix_min_score_values) =
        signal("400,500,600,700".to_string());
    let (autotune_top_k_values, set_autotune_top_k_values) = signal("4,5,6,7,8".to_string());
    let (autotune_min_score_values, set_autotune_min_score_values) =
        signal("400,500,600,700".to_string());
    let (autotune_glossary_values, set_autotune_glossary_values) = signal("true,false".to_string());

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
                                set_history_refresh.update(|v| *v += 1);
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
        let config = current_config_stored.get_value();
        run_eval(vec![ChunkingVariant {
            label: chunking_variant_label(&config),
            config,
        }]);
    };

    let run_sweep = move |_| {
        run_eval(sweep_variants(current_config_stored.get_value()));
    };

    let run_param_matrix = move |_| {
        if eval_running.get_untracked() {
            return;
        }

        let selected_label = matrix_variant_label.get_untracked();
        let variant = matrix_variants
            .get_value()
            .into_iter()
            .find(|v| v.label == selected_label);
        let Some(variant) = variant else {
            set_events.set(vec![LogEvent {
                level: LogLevel::Error,
                message: format!("PARAM_MATRIX_FAULT: unknown variant {selected_label}"),
            }]);
            return;
        };

        let top_k_values = match parse_u32_values(&matrix_top_k_values.get_untracked(), 1, 100, 1) {
            Ok(values) => values,
            Err(e) => {
                set_events.set(vec![LogEvent {
                    level: LogLevel::Error,
                    message: format!("TOP_K_RANGE_FAULT: {e}"),
                }]);
                return;
            }
        };
        let min_score_values =
            match parse_u32_values(&matrix_min_score_values.get_untracked(), 0, 1000, 100) {
                Ok(values) => values,
                Err(e) => {
                    set_events.set(vec![LogEvent {
                        level: LogLevel::Error,
                        message: format!("MIN_SCORE_RANGE_FAULT: {e}"),
                    }]);
                    return;
                }
            };

        let include_glossary_value = include_glossary.get_untracked();
        let mut option_sets = Vec::new();
        for top_k in top_k_values {
            for min_score_milli in &min_score_values {
                option_sets.push(EvaluationRunOptions {
                    top_k,
                    min_score_milli: *min_score_milli,
                    include_glossary: include_glossary_value,
                });
            }
        }

        if option_sets.is_empty() {
            set_events.set(vec![LogEvent {
                level: LogLevel::Error,
                message: "PARAM_MATRIX_FAULT: no option combinations".into(),
            }]);
            return;
        }

        set_eval_running.set(true);
        set_eval_result.set(None);
        set_events.set(Vec::new());

        let slug = slug_value.get_value();
        let slug_for_result = slug.clone();
        spawn_local(async move {
            match start_run_evaluation_matrix(slug, variant, option_sets).await {
                Ok(info) => {
                    open_event_stream_with_done(
                        info.stream_url,
                        set_events,
                        set_eval_running,
                        move || {
                            let slug = slug_for_result.clone();
                            spawn_local(async move {
                                let result = get_latest_evaluation_result(slug)
                                    .await
                                    .map_err(|e| e.to_string())
                                    .and_then(|saved| {
                                        saved.ok_or_else(|| {
                                            "evaluation finished without saving a result"
                                                .to_string()
                                        })
                                    });
                                set_eval_result.set(Some(result));
                                set_history_refresh.update(|v| *v += 1);
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

    let run_autotune = move |_| {
        if eval_running.get_untracked() {
            return;
        }

        let top_k_values = match parse_u32_values(&autotune_top_k_values.get_untracked(), 1, 100, 1)
        {
            Ok(values) => values,
            Err(e) => {
                set_events.set(vec![LogEvent {
                    level: LogLevel::Error,
                    message: format!("AUTOTUNE_TOP_K_FAULT: {e}"),
                }]);
                return;
            }
        };
        let min_score_milli_values =
            match parse_u32_values(&autotune_min_score_values.get_untracked(), 0, 1000, 100) {
                Ok(values) => values,
                Err(e) => {
                    set_events.set(vec![LogEvent {
                        level: LogLevel::Error,
                        message: format!("AUTOTUNE_MIN_SCORE_FAULT: {e}"),
                    }]);
                    return;
                }
            };
        let include_glossary_values =
            match parse_bool_values(&autotune_glossary_values.get_untracked()) {
                Ok(values) => values,
                Err(e) => {
                    set_events.set(vec![LogEvent {
                        level: LogLevel::Error,
                        message: format!("AUTOTUNE_GLOSSARY_FAULT: {e}"),
                    }]);
                    return;
                }
            };

        set_eval_running.set(true);
        set_eval_result.set(None);
        set_events.set(Vec::new());

        let slug = slug_value.get_value();
        let slug_for_result = slug.clone();
        let request = EvaluationAutotuneRequest {
            current_config: current_config_stored.get_value(),
            top_k_values,
            min_score_milli_values,
            include_glossary_values,
        };

        spawn_local(async move {
            match start_run_evaluation_autotune(slug, request).await {
                Ok(info) => {
                    open_event_stream_with_done(
                        info.stream_url,
                        set_events,
                        set_eval_running,
                        move || {
                            let slug = slug_for_result.clone();
                            spawn_local(async move {
                                let result = get_latest_evaluation_result(slug)
                                    .await
                                    .map_err(|e| e.to_string())
                                    .and_then(|saved| {
                                        saved.ok_or_else(|| {
                                            "autotune finished without saving a result".to_string()
                                        })
                                    });
                                set_eval_result.set(Some(result));
                                set_history_refresh.update(|v| *v += 1);
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

    let close_dialog = move |_| {
        if running.get() || eval_running.get() {
            return;
        }
        set_open.set(false);
    };

    view! {
        <Show when=move || open.get()>
            <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
                <div class="card-outer p-6 w-full max-w-4xl mx-4 flex flex-col gap-4 max-h-[90vh]">
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

                    <div class="flex gap-1 border-b border-[var(--color-border)]">
                        <LabTabButton
                            label="DATASET"
                            active=move || active_tab.get() == EvaluationLabTab::Dataset
                            on_click=Box::new(move || set_active_tab.set(EvaluationLabTab::Dataset))
                        />
                        <LabTabButton
                            label="SWEEP"
                            active=move || active_tab.get() == EvaluationLabTab::Sweep
                            on_click=Box::new(move || set_active_tab.set(EvaluationLabTab::Sweep))
                        />
                        <LabTabButton
                            label="MATRIX"
                            active=move || active_tab.get() == EvaluationLabTab::Matrix
                            on_click=Box::new(move || set_active_tab.set(EvaluationLabTab::Matrix))
                        />
                        <LabTabButton
                            label="AUTOTUNE"
                            active=move || active_tab.get() == EvaluationLabTab::Autotune
                            on_click=Box::new(move || set_active_tab.set(EvaluationLabTab::Autotune))
                        />
                    </div>

                    <div class="grid grid-cols-1 lg:grid-cols-[minmax(0,1fr)_minmax(320px,420px)] gap-6 overflow-auto py-2">
                        <div class="min-h-[360px]">
                            {move || match active_tab.get() {
                                EvaluationLabTab::Dataset => view! {
                                    <div class="space-y-4">
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
                                }.into_any(),
                                EvaluationLabTab::Sweep => view! {
                                    <div class="space-y-4">
                                        <span class="tech-label opacity-60">"SWEEP_EVALUATION"</span>
                                        <EvaluationOptionFields
                                            top_k=top_k
                                            set_top_k=set_top_k
                                            min_score_milli=min_score_milli
                                            set_min_score_milli=set_min_score_milli
                                            include_glossary=include_glossary
                                            set_include_glossary=set_include_glossary
                                        />
                                        <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 pt-2">
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
                                }.into_any(),
                                EvaluationLabTab::Matrix => view! {
                                    <div class="space-y-4">
                                        <span class="tech-label opacity-60">"MATRIX_EVALUATION"</span>
                                        <EvaluationOptionFields
                                            top_k=top_k
                                            set_top_k=set_top_k
                                            min_score_milli=min_score_milli
                                            set_min_score_milli=set_min_score_milli
                                            include_glossary=include_glossary
                                            set_include_glossary=set_include_glossary
                                        />
                                        <MatrixOptionFields
                                            variants=matrix_variants.get_value()
                                            matrix_variant_label=matrix_variant_label
                                            set_matrix_variant_label=set_matrix_variant_label
                                            matrix_top_k_values=matrix_top_k_values
                                            set_matrix_top_k_values=set_matrix_top_k_values
                                            matrix_min_score_values=matrix_min_score_values
                                            set_matrix_min_score_values=set_matrix_min_score_values
                                        />
                                        <button
                                            type="button"
                                            class="btn btn-primary w-full justify-center"
                                            disabled=move || eval_running.get() || running.get()
                                            on:click=run_param_matrix
                                        >
                                            "RUN_MATRIX"
                                        </button>
                                    </div>
                                }.into_any(),
                                EvaluationLabTab::Autotune => view! {
                                    <div class="space-y-4">
                                        <span class="tech-label opacity-60">"AUTOTUNE_EVALUATION"</span>
                                        <AutotuneOptionFields
                                            autotune_top_k_values=autotune_top_k_values
                                            set_autotune_top_k_values=set_autotune_top_k_values
                                            autotune_min_score_values=autotune_min_score_values
                                            set_autotune_min_score_values=set_autotune_min_score_values
                                            autotune_glossary_values=autotune_glossary_values
                                            set_autotune_glossary_values=set_autotune_glossary_values
                                        />
                                        <button
                                            type="button"
                                            class="btn btn-primary w-full justify-center"
                                            disabled=move || eval_running.get() || running.get()
                                            on:click=run_autotune
                                        >
                                            "RUN_AUTOTUNE"
                                        </button>
                                    </div>
                                }.into_any(),
                            }}
                        </div>

                        <ExecutionLog events=events eval_running=eval_running />
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
fn LabTabButton<F>(
    label: &'static str,
    active: F,
    on_click: Box<dyn Fn() + Send + Sync>,
) -> impl IntoView
where
    F: Fn() -> bool + Send + Sync + 'static,
{
    let cls = move || {
        if active() {
            "px-5 py-2 border-b-2 border-[var(--color-accent)] tech-label font-bold text-[var(--color-accent)]"
        } else {
            "px-5 py-2 border-b-2 border-transparent tech-label opacity-50 hover:opacity-100 hover:border-[var(--color-border)]"
        }
    };
    let on_click_stored = StoredValue::new(on_click);
    view! {
        <button type="button" class=cls on:click=move |_| on_click_stored.with_value(|f| f())>
            {label}
        </button>
    }
}

#[component]
fn EvaluationOptionFields(
    top_k: ReadSignal<u32>,
    set_top_k: WriteSignal<u32>,
    min_score_milli: ReadSignal<u32>,
    set_min_score_milli: WriteSignal<u32>,
    include_glossary: ReadSignal<bool>,
    set_include_glossary: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
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
    }
}

#[component]
fn MatrixOptionFields(
    variants: Vec<ChunkingVariant>,
    matrix_variant_label: ReadSignal<String>,
    set_matrix_variant_label: WriteSignal<String>,
    matrix_top_k_values: ReadSignal<String>,
    set_matrix_top_k_values: WriteSignal<String>,
    matrix_min_score_values: ReadSignal<String>,
    set_matrix_min_score_values: WriteSignal<String>,
) -> impl IntoView {
    let variants = StoredValue::new(variants);
    view! {
        <div class="space-y-3 pt-4 border-t border-[var(--color-border)]">
            <div class="tech-label opacity-60">"PARAMETER_GRID"</div>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                <SmallField label="CHUNKER">
                    <select
                        class="input font-mono text-xs"
                        prop:value=move || matrix_variant_label.get()
                        on:change=move |e| set_matrix_variant_label.set(event_target_value(&e))
                    >
                        {variants
                            .get_value()
                            .into_iter()
                            .map(|variant| {
                                let label = variant.label;
                                view! { <option value=label.clone()>{label.clone()}</option> }
                            })
                            .collect_view()}
                    </select>
                </SmallField>
                <SmallField label="TOP_K_VALUES">
                    <input
                        class="input font-mono text-xs"
                        type="text"
                        prop:value=move || matrix_top_k_values.get()
                        on:input=move |e| set_matrix_top_k_values.set(event_target_value(&e))
                    />
                </SmallField>
                <SmallField label="MIN_SCORE_VALUES">
                    <input
                        class="input font-mono text-xs"
                        type="text"
                        prop:value=move || matrix_min_score_values.get()
                        on:input=move |e| set_matrix_min_score_values.set(event_target_value(&e))
                    />
                </SmallField>
            </div>
        </div>
    }
}

#[component]
fn AutotuneOptionFields(
    autotune_top_k_values: ReadSignal<String>,
    set_autotune_top_k_values: WriteSignal<String>,
    autotune_min_score_values: ReadSignal<String>,
    set_autotune_min_score_values: WriteSignal<String>,
    autotune_glossary_values: ReadSignal<String>,
    set_autotune_glossary_values: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-3 pt-4 border-t border-[var(--color-border)]">
            <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                <SmallField label="TOP_K_VALUES">
                    <input
                        class="input font-mono text-xs"
                        type="text"
                        prop:value=move || autotune_top_k_values.get()
                        on:input=move |e| set_autotune_top_k_values.set(event_target_value(&e))
                    />
                </SmallField>
                <SmallField label="MIN_SCORE_VALUES">
                    <input
                        class="input font-mono text-xs"
                        type="text"
                        prop:value=move || autotune_min_score_values.get()
                        on:input=move |e| set_autotune_min_score_values.set(event_target_value(&e))
                    />
                </SmallField>
                <SmallField label="GLOSSARY_VALUES">
                    <input
                        class="input font-mono text-xs"
                        type="text"
                        prop:value=move || autotune_glossary_values.get()
                        on:input=move |e| set_autotune_glossary_values.set(event_target_value(&e))
                    />
                </SmallField>
            </div>
        </div>
    }
}

#[component]
fn ExecutionLog(
    events: ReadSignal<Vec<LogEvent>>,
    eval_running: ReadSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="space-y-4 flex flex-col min-h-0 h-full">
            <span class="tech-label opacity-60">"EXECUTION_LOG"</span>
            <div class="flex-1 bg-black/20 min-h-[320px] flex flex-col border border-[var(--color-border)]">
                <div class="flex-1 min-h-0 overflow-hidden">
                    <LogPanel events=events />
                </div>
                {move || eval_running.get().then(|| view! {
                    <div class="shrink-0 p-2 tech-label animate-pulse text-emerald-500 border-t border-[var(--color-border)] bg-black/30">
                        "RUNNING_EVALUATION..."
                    </div>
                })}
            </div>
        </div>
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

fn parse_u32_values(
    input: &str,
    min: u32,
    max: u32,
    default_step: u32,
) -> Result<Vec<u32>, String> {
    let mut values = Vec::new();
    for token in input.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        if let Some((range, step)) = token.split_once(':') {
            let step = step
                .parse::<u32>()
                .map_err(|_| format!("invalid step in '{token}'"))?;
            push_range_values(&mut values, range, min, max, step)?;
        } else if token.contains('-') {
            push_range_values(&mut values, token, min, max, default_step)?;
        } else {
            let value = parse_bounded_u32(token, min, max)?;
            values.push(value);
        }
    }
    values.sort_unstable();
    values.dedup();
    if values.is_empty() {
        Err("no values supplied".into())
    } else {
        Ok(values)
    }
}

fn push_range_values(
    values: &mut Vec<u32>,
    range: &str,
    min: u32,
    max: u32,
    step: u32,
) -> Result<(), String> {
    if step == 0 {
        return Err("range step must be greater than 0".into());
    }
    let (start, end) = range
        .split_once('-')
        .ok_or_else(|| format!("invalid range '{range}'"))?;
    let start = parse_bounded_u32(start.trim(), min, max)?;
    let end = parse_bounded_u32(end.trim(), min, max)?;
    if start > end {
        return Err(format!("range start is greater than end in '{range}'"));
    }
    let mut value = start;
    while value <= end {
        values.push(value);
        match value.checked_add(step) {
            Some(next) => value = next,
            None => break,
        }
    }
    Ok(())
}

fn parse_bounded_u32(token: &str, min: u32, max: u32) -> Result<u32, String> {
    let value = token
        .parse::<u32>()
        .map_err(|_| format!("invalid number '{token}'"))?;
    if value < min || value > max {
        Err(format!("{value} is outside {min}..={max}"))
    } else {
        Ok(value)
    }
}

fn parse_bool_values(input: &str) -> Result<Vec<bool>, String> {
    let mut values = Vec::new();
    for token in input.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        let value = match token.to_ascii_lowercase().as_str() {
            "true" | "t" | "1" | "yes" => true,
            "false" | "f" | "0" | "no" => false,
            _ => return Err(format!("invalid boolean '{token}'")),
        };
        values.push(value);
    }
    values.dedup();
    if values.is_empty() {
        Err("no values supplied".into())
    } else {
        Ok(values)
    }
}
