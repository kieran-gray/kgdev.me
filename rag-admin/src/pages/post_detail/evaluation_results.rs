use crate::pages::post_detail::utils::chunking_variant_label;
use crate::server_functions::chunking::save_post_chunking_config;
use crate::server_functions::settings::{load_settings, save_settings};
use crate::shared::{
    evaluation_score, ChunkingConfig, EvaluationQuestionResult, EvaluationResultSplit,
    EvaluationRunOptions, EvaluationRunResult, EvaluationVariantResult,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn EvaluationResults(
    result: EvaluationRunResult,
    slug: String,
    current_config: ChunkingConfig,
    set_override_config: WriteSignal<Option<ChunkingConfig>>,
    set_save_status: WriteSignal<Option<(bool, String)>>,
) -> impl IntoView {
    let run_options = result.options.clone();
    let autotune_summary = result.autotune.clone();
    let is_autotune = autotune_summary.is_some();
    let mut ranked_variants = result.variants;
    ranked_variants.sort_by(|a, b| compare_variants(a, b, is_autotune));
    let best_summary = ranked_variants.first().map(best_variant_summary);
    let best_action = selected_action_variant(&ranked_variants).map(best_variant_action);
    let variant_count = ranked_variants.len();
    let is_matrix = ranked_variants
        .iter()
        .any(|variant| variant.options != run_options);
    let run_detail = if let Some(summary) = &autotune_summary {
        format!(
            "AUTOTUNE_CANDIDATES={} · TUNING_Q={} · HOLDOUT_Q={}",
            summary.candidate_count, summary.tuning_question_count, summary.holdout_question_count
        )
    } else if is_matrix {
        format!("MATRIX_RESULTS={variant_count}")
    } else {
        format!(
            "TOP_K={} · MIN_SCORE={} · VARIANTS={}",
            run_options.top_k, run_options.min_score_milli, variant_count
        )
    };
    let slug = StoredValue::new(slug);
    let best_action = StoredValue::new(best_action);
    let (help_open, set_help_open) = signal(false);
    let save_best = move |_| {
        set_save_status.set(None);
        let slug = slug.get_value();
        best_action.with_value(|action| {
            let Some(action) = action.clone() else {
                return;
            };
            spawn_local(async move {
                match save_selected_configuration(slug, action.config, action.options).await {
                    Ok(()) => {
                        set_save_status.set(Some((true, format!("BEST_SAVED: {}", action.label))))
                    }
                    Err(e) => set_save_status.set(Some((false, format!("SAVE_FAULT: {e}")))),
                }
            });
        });
    };

    view! {
        <div class="space-y-4">
            <Show when=move || help_open.get()>
                <EvaluationHelpDialog set_open=set_help_open />
            </Show>

            <div class="flex flex-col md:flex-row md:items-start md:justify-between gap-3">
                <div class="flex flex-col">
                    <span class="tech-label">"evaluation.results"</span>
                    <span class="tech-label opacity-50">
                        {run_detail}
                    </span>
                </div>
                <div class="flex gap-2 self-start">
                    <button
                        type="button"
                        class="btn btn-primary px-3 py-1 text-xs"
                        title="Save the highest ranked chunking config and evaluation defaults"
                        on:click=save_best
                    >
                        "SAVE_BEST"
                    </button>
                    <button
                        type="button"
                        class="btn px-3 py-1 text-xs"
                        title="Explain evaluation metrics"
                        aria-label="Explain evaluation metrics"
                        on:click=move |_| set_help_open.set(true)
                    >
                        "?"
                    </button>
                </div>
            </div>

            {best_summary.map(|summary| view! {
                <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                    <div class="card-inner p-3 border-l-2 border-l-[var(--color-success)]">
                        <div class="tech-label opacity-50">"BEST_VARIANT"</div>
                        <div class="font-mono text-sm font-bold text-[var(--color-success)]">{summary.label}</div>
                    </div>
                    <div class="card-inner p-3">
                        <div class="tech-label opacity-50">"BALANCED_SCORE"</div>
                        <div class="font-mono text-sm font-bold">{fmt_score(summary.score)}</div>
                    </div>
                </div>
            })}

            {autotune_summary.map(|summary| view! {
                <div class="grid grid-cols-1 md:grid-cols-4 gap-3">
                    <div class="card-inner p-3 border-l-2 border-l-[var(--color-success)]">
                        <div class="tech-label opacity-50">"AUTOTUNE_SELECTED"</div>
                        <div class="font-mono text-sm font-bold text-[var(--color-success)]">{summary.selected_label}</div>
                    </div>
                    <div class="card-inner p-3">
                        <div class="tech-label opacity-50">"TUNING_SCORE"</div>
                        <div class="font-mono text-sm font-bold">{fmt_score(summary.tuning_score)}</div>
                    </div>
                    <div class="card-inner p-3">
                        <div class="tech-label opacity-50">"HOLDOUT_SCORE"</div>
                        <div class="font-mono text-sm font-bold">{fmt_score(summary.holdout_score)}</div>
                    </div>
                    <div class="card-inner p-3">
                        <div class="tech-label opacity-50">"SELECTED_PARAMS"</div>
                        <div class="font-mono text-sm font-bold">
                            {format!("k={} min={}", summary.selected_options.top_k, summary.selected_options.min_score_milli)}
                        </div>
                    </div>
                </div>
            })}

            <div class="overflow-auto border border-[var(--color-border)]">
                <table class="w-full text-[10px] border-collapse">
                    <thead>
                        <tr style="background-color: var(--color-card-inner);">
                            <th class="text-left px-2 py-2 tech-label border-b border-[var(--color-border)]">"VARIANT"</th>
                            <th class="text-right px-2 py-2 tech-label border-b border-[var(--color-border)]">"TOP_K"</th>
                            <th class="text-right px-2 py-2 tech-label border-b border-[var(--color-border)]">"MIN"</th>
                            <th class="text-right px-2 py-2 tech-label border-b border-[var(--color-border)]">"SCORE"</th>
                            <th class="text-right px-2 py-2 tech-label border-b border-[var(--color-border)]">"RECALL"</th>
                            <th class="text-right px-2 py-2 tech-label border-b border-[var(--color-border)]">"PRECISION"</th>
                            <th class="text-right px-2 py-2 tech-label border-b border-[var(--color-border)]">"IOU"</th>
                            <th class="text-right px-2 py-2 tech-label border-b border-[var(--color-border)]">"P_OMEGA"</th>
                            <th class="text-right px-2 py-2 tech-label border-b border-[var(--color-border)]">"MISSES"</th>
                            <th class="text-right px-2 py-2 tech-label border-b border-[var(--color-border)]">"CHUNKS"</th>
                            <th class="text-right px-2 py-2 tech-label border-b border-[var(--color-border)]">"AVG_CTX"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {ranked_variants
                            .into_iter()
                            .enumerate()
                            .map(|(rank, v)| {
                                view! {
                                    <EvaluationResultRow
                                        variant=v
                                        rank=rank + 1
                                        slug=slug.get_value()
                                        current_config=current_config
                                        set_override_config=set_override_config
                                        set_save_status=set_save_status
                                    />
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

#[component]
fn EvaluationResultRow(
    variant: EvaluationVariantResult,
    rank: usize,
    slug: String,
    current_config: ChunkingConfig,
    set_override_config: WriteSignal<Option<ChunkingConfig>>,
    set_save_status: WriteSignal<Option<(bool, String)>>,
) -> impl IntoView {
    let metrics = variant.metrics;
    let options = variant.options.clone();
    let options_for_save = options.clone();
    let config = variant.variant.config;
    let label = display_variant_label(variant.variant.label, &config);
    let chunk_count = variant.chunk_count;
    let average_retrieved_tokens = variant.average_retrieved_tokens;
    let missed_questions = variant
        .question_results
        .into_iter()
        .filter(|question| question.recall < 0.999 || question.missed_reference_count > 0)
        .collect::<Vec<_>>();
    let missed_count = missed_questions.len();
    let missed_questions = StoredValue::new(missed_questions);
    let slug = StoredValue::new(slug);
    let preview_label = label.clone();
    let score = evaluation_score(&metrics);
    let is_best = rank == 1;
    let is_current = config == current_config;
    let row_class = if is_best {
        "bg-emerald-950/30 hover:bg-emerald-950/40 group"
    } else {
        "hover:bg-[var(--color-card-inner)] group"
    };

    let preview = move |_| {
        set_override_config.set(Some(config));
        set_save_status.set(Some((true, format!("PREVIEWING_VARIANT: {preview_label}"))));
    };
    let save = move |_| {
        set_save_status.set(None);
        let slug = slug.get_value();
        let options = options_for_save.clone();
        spawn_local(async move {
            match save_selected_configuration(slug, config, options).await {
                Ok(()) => {
                    set_save_status.set(Some((true, "CONFIG_AND_EVAL_DEFAULTS_SAVED".into())))
                }
                Err(e) => set_save_status.set(Some((false, format!("SAVE_FAULT: {e}")))),
            }
        });
    };
    let (misses_open, set_misses_open) = signal(false);
    let toggle_misses = move |_| {
        if missed_count > 0 {
            set_misses_open.update(|open| *open = !*open);
        }
    };

    view! {
        <>
            <tr class=row_class>
                <td class="px-2 py-2 font-mono border-b border-[var(--color-border)]">
                    <div class="flex flex-col gap-1">
                        <div class="flex items-center gap-2">
                            <span class=move || if is_best { "font-bold text-[var(--color-success)]" } else { "font-bold" }>
                                {format!("#{} {}", rank, label.clone())}
                            </span>
                            {is_best.then(|| view! { <span class="badge !text-emerald-300 !border-emerald-500 !bg-emerald-950/70">"BEST"</span> })}
                            {is_current.then(|| view! { <span class="badge !text-sky-300 !border-sky-500 !bg-sky-950/70">"CURRENT"</span> })}
                        </div>
                        <div class="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                            <button type="button" class="tech-label !text-emerald-500 hover:underline" on:click=preview>
                                "PREVIEW"
                            </button>
                            <button type="button" class="tech-label !text-amber-500 hover:underline" on:click=save>
                                "SAVE"
                            </button>
                        </div>
                    </div>
                </td>
                <td class="px-2 py-2 font-mono text-right border-b border-[var(--color-border)]">
                    {options.top_k}
                </td>
                <td class="px-2 py-2 font-mono text-right border-b border-[var(--color-border)]">
                    {options.min_score_milli}
                </td>
                <td class="px-2 py-2 font-mono text-right border-b border-[var(--color-border)] font-bold">
                    {fmt_score(score)}
                </td>
                <td class="px-2 py-2 font-mono text-right border-b border-[var(--color-border)]">
                    {fmt_score(metrics.recall_mean)}
                </td>
                <td class="px-2 py-2 font-mono text-right border-b border-[var(--color-border)]">
                    {fmt_score(metrics.precision_mean)}
                </td>
                <td class="px-2 py-2 font-mono text-right border-b border-[var(--color-border)]">
                    {fmt_score(metrics.iou_mean)}
                </td>
                <td class="px-2 py-2 font-mono text-right border-b border-[var(--color-border)]">
                    {fmt_score(metrics.precision_omega_mean)}
                </td>
                <td class="px-2 py-2 font-mono text-right border-b border-[var(--color-border)]">
                    <button
                        type="button"
                        class=move || {
                            if missed_count > 0 {
                                "tech-label !text-amber-400 hover:underline"
                            } else {
                                "tech-label opacity-30"
                            }
                        }
                        disabled=missed_count == 0
                        on:click=toggle_misses
                    >
                        {missed_count}
                    </button>
                </td>
                <td class="px-2 py-2 font-mono text-right border-b border-[var(--color-border)]">
                    {chunk_count}
                </td>
                <td class="px-2 py-2 font-mono text-right border-b border-[var(--color-border)]">
                    {average_retrieved_tokens}
                </td>
            </tr>
            <Show when=move || { misses_open.get() && missed_count > 0 }>
                <tr>
                    <td colspan="11" class="p-0 border-b border-[var(--color-border)] bg-black/20">
                        <MissedQuestionDetails missed_questions=missed_questions />
                    </td>
                </tr>
            </Show>
        </>
    }
}

#[component]
fn MissedQuestionDetails(
    missed_questions: StoredValue<Vec<EvaluationQuestionResult>>,
) -> impl IntoView {
    view! {
        <div class="p-3 space-y-3">
            {missed_questions
                .with_value(|questions| {
                    questions
                        .iter()
                        .take(20)
                        .map(|question| {
                            let missed_references = question
                                .reference_results
                                .iter()
                                .filter(|reference| reference.recall < 0.999)
                                .map(|reference| {
                                    view! {
                                        <div class="border border-[var(--color-border)] p-2">
                                            <div class="tech-label opacity-50 mb-1">
                                                {format!(
                                                    "REFERENCE_RECALL={} · COVERED={}/{} CHARS · RANGE={}-{}",
                                                    fmt_score(reference.recall),
                                                    reference.covered_chars,
                                                    reference.total_chars,
                                                    reference.char_start,
                                                    reference.char_end,
                                                )}
                                            </div>
                                            <pre class="log-pre text-[10px] bg-transparent border-none p-0 max-h-[8rem]">
                                                {truncate_chars(&reference.content, 700)}
                                            </pre>
                                        </div>
                                    }
                                })
                                .collect_view();
                            let fallback = question.reference_results.is_empty().then(|| {
                                view! {
                                    <div class="tech-label opacity-50">
                                        "REFERENCE_DETAILS_UNAVAILABLE_FOR_THIS_SAVED_RESULT"
                                    </div>
                                }
                            });
                            view! {
                                <div class="card-inner p-3 space-y-2">
                                    <div class="flex flex-col gap-1">
                                        <div class="tech-label opacity-50">
                                            {format!(
                                                "QUESTION_RECALL={} · MISSED_REFERENCES={} · RETRIEVED_CHUNKS={:?}",
                                                fmt_score(question.recall),
                                                question.missed_reference_count,
                                                question.retrieved_chunk_ids,
                                            )}
                                        </div>
                                        <div class="font-mono text-xs">{question.question.clone()}</div>
                                    </div>
                                    <div class="space-y-2">
                                        {fallback}
                                        {missed_references}
                                    </div>
                                </div>
                            }
                        })
                        .collect_view()
                })}
        </div>
    }
}

async fn save_selected_configuration(
    slug: String,
    config: ChunkingConfig,
    options: EvaluationRunOptions,
) -> Result<(), String> {
    save_post_chunking_config(slug, config)
        .await
        .map_err(|e| e.to_string())?;

    let mut settings = load_settings().await.map_err(|e| e.to_string())?;
    settings.evaluation.top_k = options.top_k;
    settings.evaluation.min_score_milli = options.min_score_milli;
    settings.evaluation.include_glossary = options.include_glossary;
    save_settings(settings).await.map_err(|e| e.to_string())
}

#[derive(Clone)]
struct BestVariantAction {
    label: String,
    config: ChunkingConfig,
    options: EvaluationRunOptions,
}

fn best_variant_action(variant: &EvaluationVariantResult) -> BestVariantAction {
    BestVariantAction {
        label: display_variant_label(variant.variant.label.clone(), &variant.variant.config),
        config: variant.variant.config,
        options: variant.options.clone(),
    }
}

fn selected_action_variant(
    variants: &[EvaluationVariantResult],
) -> Option<&EvaluationVariantResult> {
    variants
        .iter()
        .find(|variant| variant.selected && variant.split == EvaluationResultSplit::Holdout)
        .or_else(|| variants.first())
}

fn compare_variants(
    a: &EvaluationVariantResult,
    b: &EvaluationVariantResult,
    is_autotune: bool,
) -> std::cmp::Ordering {
    if is_autotune {
        let priority = autotune_priority(b).cmp(&autotune_priority(a));
        if priority != std::cmp::Ordering::Equal {
            return priority;
        }
    }

    evaluation_score(&b.metrics)
        .partial_cmp(&evaluation_score(&a.metrics))
        .unwrap_or(std::cmp::Ordering::Equal)
}

fn autotune_priority(variant: &EvaluationVariantResult) -> u8 {
    if variant.selected && variant.split == EvaluationResultSplit::Holdout {
        2
    } else if variant.split == EvaluationResultSplit::Holdout {
        1
    } else {
        0
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

#[derive(Clone)]
struct BestVariantSummary {
    label: String,
    score: f32,
}

fn best_variant_summary(variant: &EvaluationVariantResult) -> BestVariantSummary {
    BestVariantSummary {
        label: display_variant_label(variant.variant.label.clone(), &variant.variant.config),
        score: evaluation_score(&variant.metrics),
    }
}

fn display_variant_label(label: String, config: &ChunkingConfig) -> String {
    if label == "current" {
        chunking_variant_label(config)
    } else {
        label
    }
}

fn fmt_score(value: f32) -> String {
    format!("{:.1}%", value * 100.0)
}

#[component]
fn EvaluationHelpDialog(set_open: WriteSignal<bool>) -> impl IntoView {
    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
            <div class="card-outer p-6 w-full max-w-3xl mx-4 flex flex-col gap-4 max-h-[85vh]">
                <div class="flex items-start justify-between border-b border-[var(--color-border)] pb-2">
                    <div class="flex flex-col">
                        <span class="tech-label">"evaluation.help"</span>
                        <h2 class="text-lg font-bold">"INTERPRETING_RESULTS"</h2>
                    </div>
                    <button
                        type="button"
                        class="tech-label opacity-50 hover:opacity-100 px-2 py-0.5 border border-[var(--color-border)] cursor-pointer"
                        aria-label="Close evaluation help"
                        on:click=move |_| set_open.set(false)
                    >
                        "x"
                    </button>
                </div>

                <div class="overflow-auto space-y-4 text-xs leading-relaxed">
                    <div class="card-inner p-3">
                        <div class="tech-label opacity-50 mb-1">"QUICK_READ"</div>
                        <p>
                            "Rows are sorted by BALANCED_SCORE. Use SAVE_BEST to apply the highest ranked chunking config and evaluation defaults. Check MISSES before accepting a run with imperfect recall."
                        </p>
                    </div>

                    <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                        <MetricHelp
                            name="SCORE"
                            detail="Weighted summary used for ranking: 40% recall, 25% IoU, 20% precision, 15% precision omega. Higher is better."
                        />
                        <MetricHelp
                            name="RECALL"
                            detail="How often retrieved chunks cover the reference text needed to answer the generated questions. Higher is better; low recall means the chunking strategy misses answer evidence."
                        />
                        <MetricHelp
                            name="PRECISION"
                            detail="How much retrieved text is actually relevant reference text. Higher is better; low precision means the retriever is carrying extra context."
                        />
                        <MetricHelp
                            name="IOU"
                            detail="Overlap quality between retrieved ranges and reference ranges. Higher is better; it rewards hitting the right span without too much surrounding text."
                        />
                        <MetricHelp
                            name="P_OMEGA"
                            detail="Best possible precision if retrieval selected every chunk touching a reference. Higher is better; it estimates how efficient the chunk boundaries are before ranking effects."
                        />
                        <MetricHelp
                            name="MISSES"
                            detail="Questions with incomplete reference coverage. Open this to see the missed excerpts, their source ranges, and how much of each reference was covered."
                        />
                        <MetricHelp
                            name="CHUNKS / AVG_CTX"
                            detail="CHUNKS is the number of chunks created. AVG_CTX is average retrieved tokens per question. Lower is usually cheaper and easier for the answer model, but only after recall is acceptable."
                        />
                    </div>
                </div>

                <div class="flex justify-end pt-2 border-t border-[var(--color-border)]">
                    <button type="button" class="btn px-8" on:click=move |_| set_open.set(false)>
                        "DONE"
                    </button>
                </div>
            </div>
        </div>
    }
}

#[component]
fn MetricHelp(name: &'static str, detail: &'static str) -> impl IntoView {
    view! {
        <div class="card-inner p-3">
            <div class="tech-label opacity-50 mb-1">{name}</div>
            <p>{detail}</p>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::EvaluationMetrics;

    fn metrics(
        recall_mean: f32,
        precision_mean: f32,
        iou_mean: f32,
        precision_omega_mean: f32,
    ) -> EvaluationMetrics {
        EvaluationMetrics {
            recall_mean,
            recall_std: 0.0,
            precision_mean,
            precision_std: 0.0,
            iou_mean,
            iou_std: 0.0,
            precision_omega_mean,
            precision_omega_std: 0.0,
        }
    }

    #[test]
    fn evaluation_score_prioritizes_recall_then_overlap_and_precision() {
        let balanced = metrics(0.9, 0.7, 0.8, 0.6);
        let weak_recall = metrics(0.4, 1.0, 1.0, 1.0);

        assert!(evaluation_score(&balanced) > evaluation_score(&weak_recall));
    }
}
