use leptos::prelude::*;

/// Modal dialog shell.
///
/// Visibility is driven by `open` (read signal). Backdrop click and `Esc` are
/// handled by callers (they'd close via `on_close`). The body is a re-runnable
/// closure so a single shell can host different forms across opens.
///
/// Future evolution: the UX plan calls for right-edge side-panels rather than
/// centred modals. This primitive intentionally keeps a tight API so swapping
/// the layout later doesn't ripple through every call site.
#[component]
pub fn Dialog(
    /// Controls visibility. The dialog mounts when `open()` becomes `true`.
    #[prop(into)]
    open: Signal<bool>,
    /// Required heading.
    #[prop(into)]
    title: String,
    /// One-line description rendered below the heading.
    #[prop(optional, into)]
    subtitle: Option<String>,
    /// Called when the operator dismisses the dialog (close button).
    on_close: Callback<()>,
    /// Form / content body. `ChildrenFn` so the same dialog can re-render
    /// when its content depends on signals.
    children: ChildrenFn,
) -> impl IntoView {
    let children = StoredValue::new(children);
    view! {
        <Show when=move || open.get()>
            <div
                class="fixed inset-0 z-50 flex items-start justify-center pt-20 bg-black/60 backdrop-blur-sm"
                on:click=move |ev| {
                    // Click on the backdrop (not its children) closes.
                    let target = ev.target();
                    let current = ev.current_target();
                    if target == current {
                        on_close.run(());
                    }
                }
            >
                <div class="surface w-full max-w-lg mx-4 max-h-[80vh] flex flex-col overflow-hidden">
                    <div class="flex items-start justify-between gap-3 px-5 py-4 border-b border-[var(--color-border)]">
                        <div class="min-w-0">
                            <h2 class="section-title">{title.clone()}</h2>
                            {subtitle.clone().map(|s| view! { <p class="muted text-sm mt-1">{s}</p> })}
                        </div>
                        <button
                            type="button"
                            class="btn-ghost btn"
                            aria-label="Close"
                            on:click=move |_| on_close.run(())
                        >
                            "✕"
                        </button>
                    </div>
                    <div class="p-5 overflow-y-auto">
                        {children.with_value(|c| c())}
                    </div>
                </div>
            </div>
        </Show>
    }
}
