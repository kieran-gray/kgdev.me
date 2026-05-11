use leptos::prelude::*;

use crate::components::primitives::{EmptyState, PageHeader, Surface};

/// Cross-document evaluation history & best-variant leaderboard.
///
/// Stubbed at this stage: a future commit wires up the cross-document run
/// projection (best variant per document, paged run list). The page exists now
/// so navigation, breadcrumbs, and event-bus invalidation seams can be wired
/// without a route hole.
#[component]
pub fn EvaluationsPage() -> impl IntoView {
    view! {
        <div>
            <PageHeader
                title="Evaluations"
                subtitle="Cross-document run history and best-variant leaderboard.".to_string()
            />
            <Surface title="Best variants".to_string()>
                <EmptyState
                    title="No evaluation runs yet"
                    body="Open a document and run an evaluation to populate the leaderboard.".to_string()
                />
            </Surface>
        </div>
    }
}
