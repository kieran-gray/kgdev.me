use leptos::prelude::*;

/// Semantic status states used across the app. The mapping from domain status
/// strings (e.g. `IndexingReadModel.status`) to `Status` lives at the call site
/// so domain semantics aren't trapped inside a presentation component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Successful terminal state — Indexed, Completed, Ready.
    Ok,
    /// Active progress — Pending, Chunking, Embedding, Generating.
    Pending,
    /// Terminal failure — Failed.
    Fail,
    /// Source has moved on past the deployed indexing.
    Stale,
    /// Informational — neutral colour with a leading dot.
    Info,
    /// Default neutral — no leading dot, faded text.
    Neutral,
}

impl Status {
    fn class(self) -> &'static str {
        match self {
            Self::Ok => "pill pill-ok",
            Self::Pending => "pill pill-pending",
            Self::Fail => "pill pill-fail",
            Self::Stale => "pill pill-stale",
            Self::Info => "pill pill-info",
            Self::Neutral => "pill pill-neutral",
        }
    }
}

#[component]
pub fn StatusPill(
    /// Display label for the pill.
    #[prop(into)]
    label: String,
    /// Semantic colour bucket.
    #[prop(optional, into)]
    kind: Option<Status>,
) -> impl IntoView {
    let kind = kind.unwrap_or(Status::Neutral);
    view! { <span class=kind.class()>{label}</span> }
}
