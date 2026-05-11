use leptos::prelude::*;

use super::nav::AppNav;

/// Top-level layout: nav header + centred content column.
///
/// Replaces `components/layout.rs::Layout` with a calmer, framing-line-free
/// shell aligned with the refined dev-tool aesthetic.
#[component]
pub fn AppShell(children: Children) -> impl IntoView {
    view! {
        <div class="min-h-screen flex flex-col bg-[var(--color-page-bg)]">
            <AppNav />
            <main class="flex-1 w-full">
                <div class="max-w-6xl mx-auto px-6 py-8">{children()}</div>
            </main>
        </div>
    }
}
