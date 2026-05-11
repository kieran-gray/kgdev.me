use leptos::prelude::*;

/// Labelled key/value row used in detail panels. Renders the key as a muted
/// fixed-width column and the value mono-aligned next to it.
#[component]
pub fn Kv(
    #[prop(into)] label: String,
    #[prop(optional, into)] value: Option<String>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="flex items-baseline gap-3 text-sm">
            <span class="eyebrow w-28 shrink-0">{label}</span>
            <span class="text-text">
                {match (value, children) {
                    (Some(v), _) => view! { <span>{v}</span> }.into_any(),
                    (None, Some(c)) => view! { <span>{c()}</span> }.into_any(),
                    (None, None) => view! { <span class="faint">"—"</span> }.into_any(),
                }}
            </span>
        </div>
    }
}
