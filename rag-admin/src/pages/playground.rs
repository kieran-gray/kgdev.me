use leptos::prelude::*;

use crate::components::primitives::{EmptyState, PageHeader, Surface};

/// RAG query playground.
///
/// Stubbed at this stage. The full design (top-k, min-score, history,
/// per-result jump-to-source) lands once the `query_documents` server function
/// exists.
#[component]
pub fn PlaygroundPage() -> impl IntoView {
    view! {
        <div>
            <PageHeader
                title="Playground"
                subtitle="Test retrieval against any pipeline. Results route back to the source document for chunking iteration.".to_string()
            />
            <Surface title="Query".to_string()>
                <EmptyState
                    title="Backend not yet available"
                    body="The query_documents server function will land in a follow-up. The UI is ready to wire to it.".to_string()
                />
            </Surface>
        </div>
    }
}
