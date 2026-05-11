use leptos::prelude::*;

/// Bordered card with optional title and toolbar slots.
///
/// Replaces the ad-hoc `card-outer p-4 space-y-3` pattern. Use `Surface` for
/// every grouped region. Nesting is expressed via `raised`, not via a second
/// background colour.
#[component]
pub fn Surface(
    /// Optional heading rendered in the surface's top row.
    #[prop(optional, into)]
    title: Option<String>,
    /// Optional trailing slot to the right of the title (action buttons, badges).
    #[prop(optional)]
    actions: Option<Children>,
    /// When true, uses the raised surface colour (for nested surfaces).
    #[prop(optional)]
    raised: bool,
    /// When true, removes interior padding so the caller can render edge-to-edge
    /// content (e.g. a `DataTable`).
    #[prop(optional)]
    flush: bool,
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let base = if raised { "surface-raised" } else { "surface" };
    let padding = if flush { "" } else { "p-4" };
    let extra = class.unwrap_or_default();
    let class = format!("{base} {padding} {extra}");

    let header = match (title.as_ref(), actions.is_some()) {
        (None, false) => None,
        _ => Some(view! {
            <div class="flex items-center justify-between mb-3 gap-3">
                <div class="section-title">{title.clone().unwrap_or_default()}</div>
                <div class="flex items-center gap-2">
                    {actions.map(|c| c())}
                </div>
            </div>
        }),
    };

    view! {
        <section class=class>
            {header}
            {children()}
        </section>
    }
}
