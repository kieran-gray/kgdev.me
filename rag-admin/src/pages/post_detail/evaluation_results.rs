use crate::server_fns::save_post_chunking_config;
use crate::shared::{
    ChunkingConfig, EvaluationMetrics, EvaluationRunResult, EvaluationVariantResult,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

const RECALL_WEIGHT: f32 = 0.40;
const IOU_WEIGHT: f32 = 0.25;
const PRECISION_WEIGHT: f32 = 0.20;
const PRECISION_OMEGA_WEIGHT: f32 = 0.15;

#[component]
pub fn EvaluationResults(
    result: EvaluationRunResult,
    slug: String,
    set_override_config: WriteSignal<Option<ChunkingConfig>>,
    set_save_status: WriteSignal<Option<(bool, String)>>,
) -> impl IntoView {
    let mut ranked_variants = result.variants;
    ranked_variants.sort_by(|a, b| {
        evaluation_score(&b.metrics)
            .partial_cmp(&evaluation_score(&a.metrics))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let best_summary = ranked_variants.first().map(best_variant_summary);
    let variant_count = ranked_variants.len();
    let slug = StoredValue::new(slug);
    let (help_open, set_help_open) = signal(false);

    view! {
        <div class="space-y-4">
            <Show when=move || help_open.get()>
                <EvaluationHelpDialog set_open=set_help_open />
            </Show>

            <div class="flex flex-col md:flex-row md:items-start md:justify-between gap-3">
                <div class="flex flex-col">
                    <span class="tech-label">"evaluation.results"</span>
                    <span class="tech-label opacity-50">
                        {format!("TOP_K={} · MIN_SCORE={} · VARIANTS={}", result.options.top_k, result.options.min_score_milli, variant_count)}
                    </span>
                </div>
                <button
                    type="button"
                    class="btn px-3 py-1 text-xs self-start"
                    title="Explain evaluation metrics"
                    aria-label="Explain evaluation metrics"
                    on:click=move |_| set_help_open.set(true)
                >
                    "?"
                </button>
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

            <div class="overflow-auto border border-[var(--color-border)]">
                <table class="w-full text-[10px] border-collapse">
                    <thead>
                        <tr style="background-color: var(--color-card-inner);">
                            <th class="text-left px-2 py-2 tech-label border-b border-[var(--color-border)]">"VARIANT"</th>
                            <th class="text-right px-2 py-2 tech-label border-b border-[var(--color-border)]">"SCORE"</th>
                            <th class="text-right px-2 py-2 tech-label border-b border-[var(--color-border)]">"RECALL"</th>
                            <th class="text-right px-2 py-2 tech-label border-b border-[var(--color-border)]">"PRECISION"</th>
                            <th class="text-right px-2 py-2 tech-label border-b border-[var(--color-border)]">"IOU"</th>
                            <th class="text-right px-2 py-2 tech-label border-b border-[var(--color-border)]">"P_OMEGA"</th>
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
    set_override_config: WriteSignal<Option<ChunkingConfig>>,
    set_save_status: WriteSignal<Option<(bool, String)>>,
) -> impl IntoView {
    let metrics = variant.metrics;
    let label = variant.variant.label;
    let config = variant.variant.config;
    let slug = StoredValue::new(slug);
    let preview_label = label.clone();
    let score = evaluation_score(&metrics);
    let is_best = rank == 1;
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
        spawn_local(async move {
            match save_post_chunking_config(slug, config).await {
                Ok(()) => set_save_status.set(Some((true, "POST_CONFIG_SAVED".into()))),
                Err(e) => set_save_status.set(Some((false, format!("SAVE_FAULT: {e}")))),
            }
        });
    };

    view! {
        <tr class=row_class>
            <td class="px-2 py-2 font-mono border-b border-[var(--color-border)]">
                <div class="flex flex-col gap-1">
                    <div class="flex items-center gap-2">
                        <span class=move || if is_best { "font-bold text-[var(--color-success)]" } else { "font-bold" }>
                            {format!("#{} {}", rank, label.clone())}
                        </span>
                        {is_best.then(|| view! { <span class="badge !text-emerald-300 !border-emerald-500 !bg-emerald-950/70">"BEST"</span> })}
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
                {variant.chunk_count}
            </td>
            <td class="px-2 py-2 font-mono text-right border-b border-[var(--color-border)]">
                {variant.average_retrieved_chars}
            </td>
        </tr>
    }
}

#[derive(Clone)]
struct BestVariantSummary {
    label: String,
    score: f32,
}

fn best_variant_summary(variant: &EvaluationVariantResult) -> BestVariantSummary {
    BestVariantSummary {
        label: variant.variant.label.clone(),
        score: evaluation_score(&variant.metrics),
    }
}

fn evaluation_score(metrics: &EvaluationMetrics) -> f32 {
    metrics.recall_mean * RECALL_WEIGHT
        + metrics.iou_mean * IOU_WEIGHT
        + metrics.precision_mean * PRECISION_WEIGHT
        + metrics.precision_omega_mean * PRECISION_OMEGA_WEIGHT
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
                            "Rows are sorted by BALANCED_SCORE. Start with the BEST row, then check RECALL for missed answers and AVG_CTX/CHUNKS for retrieval cost. Prefer the top row unless another variant has similar recall with much less context."
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
                            name="CHUNKS / AVG_CTX"
                            detail="CHUNKS is the number of chunks created. AVG_CTX is average retrieved characters per question. Lower is usually cheaper and easier for the answer model, but only after recall is acceptable."
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
