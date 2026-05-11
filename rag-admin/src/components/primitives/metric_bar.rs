use leptos::prelude::*;

/// Horizontal metric bar with a numeric label, axis ticks, and optional std-dev
/// and best-in-run marker.
///
/// Designed to be scannable as part of a small-multiples table where every row
/// shares the same 0-100% axis. The bar always spans the full track so the
/// reader compares fills, not bar widths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetricKind {
    #[default]
    Default,
    /// Highlight as the headline / selected variant.
    Best,
}

#[component]
pub fn MetricBar(
    /// Full metric name shown on the left (e.g. "Recall", "Precision").
    #[prop(into)]
    label: String,
    /// One-letter scientific shorthand shown muted next to the label.
    /// Provide `None` to suppress.
    #[prop(optional, into)]
    short: Option<String>,
    /// Tooltip / `title` attribute explaining what the metric measures.
    #[prop(optional, into)]
    help: Option<String>,
    /// Value in [0.0, 1.0].
    value: f32,
    /// Optional std-dev in [0.0, 1.0]. Rendered as a ± numeric suffix and a
    /// translucent range underlay around the value marker.
    #[prop(optional)]
    stddev: Option<f32>,
    /// Optional reference value in [0.0, 1.0]: the best score across all
    /// variants in the same run. A faint tick marker is drawn at this point so
    /// the reader can see how this variant compares to the leader.
    #[prop(optional)]
    best: Option<f32>,
    /// Visual emphasis.
    #[prop(optional)]
    kind: Option<MetricKind>,
) -> impl IntoView {
    let kind = kind.unwrap_or_default();
    let value = value.clamp(0.0, 1.0);

    let bar_colour = match kind {
        MetricKind::Best => "var(--color-accent)",
        MetricKind::Default => "var(--status-info)",
    };

    let value_pct = format!("{:.1}%", value * 100.0);

    // Numeric value with std-dev as a ± suffix. Std-dev is "uncertainty across
    // the question population"; the reader cares about its magnitude, not its
    // exact bounds — a number is clearer than a whisker.
    let value_label = match stddev {
        Some(s) if s > 0.0005 => format!("{:.1}% ± {:.1}", value * 100.0, s * 100.0),
        _ => format!("{:.1}%", value * 100.0),
    };

    // Std-dev underlay: a translucent band ±σ around the value marker, clamped
    // to [0, 100]%. Anchored to the bar's track, not the fill, so the reader
    // sees it as a confidence interval.
    let stddev_band = stddev.and_then(|s| {
        if s <= 0.0005 {
            return None;
        }
        let s = s.clamp(0.0, 1.0);
        let left = ((value - s).max(0.0) * 100.0).clamp(0.0, 100.0);
        let right = ((value + s).min(1.0) * 100.0).clamp(0.0, 100.0);
        let width = (right - left).max(0.0);
        Some(view! {
            <span
                class="metric-bar-stddev"
                style=format!("left: {left:.2}%; width: {width:.2}%")
                aria-hidden="true"
            ></span>
        })
    });

    // Reference marker for the best score in the run — only render when this
    // variant is *not* the leader (otherwise the marker would sit on top of
    // the value marker).
    let best_marker = best.and_then(|b| {
        let b = b.clamp(0.0, 1.0);
        if (b - value).abs() < 0.001 {
            return None;
        }
        let left = (b * 100.0).clamp(0.0, 100.0);
        Some(view! {
            <span
                class="metric-bar-best-tick"
                style=format!("left: {left:.2}%")
                title=format!("Best in run: {:.1}%", b * 100.0)
                aria-hidden="true"
            ></span>
        })
    });

    view! {
        <div class="metric-bar-row" title=help.clone().unwrap_or_default()>
            <div class="metric-bar-label">
                <span class="metric-bar-label-name">{label}</span>
                {short.map(|s| view! {
                    <span class="metric-bar-label-short">{s}</span>
                })}
            </div>
            <div class="metric-bar-track" role="img" aria-label=value_pct>
                // Axis ticks at 25, 50, 75 — faint guides for scale.
                <span class="metric-bar-tick" style="left: 25%"></span>
                <span class="metric-bar-tick" style="left: 50%"></span>
                <span class="metric-bar-tick" style="left: 75%"></span>
                {stddev_band}
                <span
                    class="metric-bar-fill"
                    style=format!("width: {}; background-color: {}", value_pct, bar_colour)
                ></span>
                {best_marker}
            </div>
            <span class="metric-bar-value">{value_label}</span>
        </div>
    }
}
