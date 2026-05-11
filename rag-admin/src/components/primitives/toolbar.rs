use leptos::prelude::*;

/// Inline toolbar used at the top of list pages. Children are arranged in a
/// single horizontal row that wraps on narrow viewports.
///
/// Convention: the first set of children are filters / search inputs, then a
/// flexible spacer, then the primary action(s) on the right. Callers compose
/// directly — this component is a layout shell, not an opinionated widget.
#[component]
pub fn Toolbar(children: Children) -> impl IntoView {
    view! { <div class="toolbar mb-4">{children()}</div> }
}
