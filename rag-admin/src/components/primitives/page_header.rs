use leptos::prelude::*;

/// The single page heading row used by every top-level page.
///
/// Replaces the per-page `SYSTEM_VIEW / FOO`/`tech-label`/`text-3xl` triple
/// with a consistent (eyebrow, title, actions) row.
#[component]
pub fn PageHeader(
    /// Page title text.
    #[prop(into)]
    title: String,
    /// Optional small label rendered above the title (used for breadcrumbs).
    #[prop(optional, into)]
    eyebrow: Option<String>,
    /// Optional subtitle below the title (one short line).
    #[prop(optional, into)]
    subtitle: Option<String>,
    /// Right-aligned action slot (buttons, dropdowns).
    #[prop(optional)]
    actions: Option<Children>,
) -> impl IntoView {
    view! {
        <header class="flex items-end justify-between gap-4 mb-6">
            <div class="flex flex-col gap-1 min-w-0">
                {eyebrow.map(|e| view! { <div class="eyebrow">{e}</div> })}
                <h1 class="page-title truncate">{title}</h1>
                {subtitle.map(|s| view! { <p class="muted text-sm">{s}</p> })}
            </div>
            {actions.map(|a| view! {
                <div class="flex items-center gap-2 shrink-0">{a()}</div>
            })}
        </header>
    }
}
