use leptos::prelude::*;

use crate::components::primitives::{EmptyState, Surface};

/// Source tab — will render the document's markdown with span highlights for
/// hovered evaluation references. Stubbed in this slice; the markdown server
/// fn lands when the `Source` aggregate gains a `get_rendered_markdown` query.
#[component]
pub fn SourceTab(source_ref: String) -> impl IntoView {
    view! {
        <Surface title="Source content".to_string()>
            <EmptyState
                title="Not yet implemented"
                body=format!(
                    "The source view will render the markdown for {source_ref} with character-range highlighting for evaluation references. Backend query lands in a follow-up."
                )
            />
        </Surface>
    }
}
