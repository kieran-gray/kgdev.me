use leptos::prelude::*;

/// Horizontal metric bar with a numeric label.
///
/// Used in the evaluation run leaderboard to make recall/precision/IoU/Pω
/// scannable across variants. `value` is in `[0.0, 1.0]` and is displayed as a
/// percentage. `kind` picks a semantic colour; for plain progress, use
/// `Default`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetricKind {
    #[default]
    Default,
    /// Highlight as the headline / selected variant.
    Best,
}

#[component]
pub fn MetricBar(
    /// Short label (e.g. "R", "P", "IoU", "Pω").
    #[prop(into)]
    label: String,
    /// Value in [0.0, 1.0].
    value: f32,
    /// Optional std-dev whisker in [0.0, 1.0].
    #[prop(optional)]
    stddev: Option<f32>,
    /// Visual emphasis.
    #[prop(optional)]
    kind: Option<MetricKind>,
) -> impl IntoView {
    let kind = kind.unwrap_or_default();
    let value = value.clamp(0.0, 1.0);
    let pct_width = format!("{:.1}%", value * 100.0);
    let pct_label = format!("{:.1}%", value * 100.0);

    let bar_colour = match kind {
        MetricKind::Best => "var(--color-accent)",
        MetricKind::Default => "var(--status-info)",
    };

    let stddev_marker = stddev.map(|s| {
        let s = s.clamp(0.0, 1.0);
        let left = ((value - s).max(0.0) * 100.0).clamp(0.0, 100.0);
        let width = ((s * 2.0).min(1.0) * 100.0).clamp(0.0, 100.0);
        view! {
            <span
                class="absolute top-1/2 -translate-y-1/2 h-1.5 border-y border-[var(--color-text-faint)] opacity-40"
                style=format!("left: {left:.1}%; width: {width:.1}%")
            ></span>
        }
    });

    view! {
        <div class="flex items-center gap-3 text-sm">
            <span class="eyebrow w-10 shrink-0">{label}</span>
            <div class="relative flex-1 h-2 rounded bg-[var(--color-surface-2)] border border-[var(--color-border)] overflow-hidden">
                <span
                    class="absolute inset-y-0 left-0 rounded-l"
                    style=format!("width: {pct_width}; background-color: {bar_colour}; opacity: 0.85;")
                ></span>
                {stddev_marker}
            </div>
            <span class="text-text font-mono text-xs w-14 text-right">{pct_label}</span>
        </div>
    }
}
