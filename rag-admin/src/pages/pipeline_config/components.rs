use leptos::prelude::*;

#[component]
pub fn PanelHeader<F>(
    title: &'static str,
    description: &'static str,
    action_label: &'static str,
    action_disabled: F,
    on_action: Box<dyn Fn(leptos::ev::MouseEvent) + Send + Sync>,
) -> impl IntoView
where
    F: Fn() -> bool + Send + Sync + 'static,
{
    let on_action = StoredValue::new(on_action);
    view! {
        <div class="card-outer p-5 flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
            <div class="space-y-2">
                <span class="tech-label opacity-60">{title}</span>
                <p class="tech-label opacity-50 max-w-2xl">{description}</p>
            </div>
            <button
                class="btn btn-primary"
                disabled=action_disabled
                on:click=move |ev| on_action.with_value(|f| f(ev))
            >
                {action_label}
            </button>
        </div>
    }
}

#[component]
pub fn EmptyState(message: &'static str) -> impl IntoView {
    view! {
        <div class="card-outer p-6">
            <p class="tech-label opacity-50">{message}</p>
        </div>
    }
}

#[component]
pub fn MetaPill(label: String) -> impl IntoView {
    view! { <span class="badge">{label}</span> }
}

#[component]
pub fn TabButton<F>(
    label: &'static str,
    active: F,
    on_click: Box<dyn Fn() + Send + Sync>,
) -> impl IntoView
where
    F: Fn() -> bool + Send + Sync + 'static,
{
    let on_click = StoredValue::new(on_click);
    let cls = move || {
        if active() {
            "px-5 py-2 border-b-2 border-[var(--color-accent)] tech-label font-bold text-[var(--color-accent)] whitespace-nowrap"
        } else {
            "px-5 py-2 border-b-2 border-transparent tech-label opacity-50 hover:opacity-100 hover:border-[var(--color-border)] whitespace-nowrap"
        }
    };
    view! {
        <button type="button" class=cls on:click=move |_| on_click.with_value(|f| f())>
            {label}
        </button>
    }
}

#[component]
pub fn Field(label: &'static str, hint: &'static str, children: Children) -> impl IntoView {
    view! {
        <label class="block space-y-1.5">
            <div class="tech-label opacity-70">{label}</div>
            {children()}
            <div class="tech-label text-[9px] opacity-40">{format!("> {}", hint)}</div>
        </label>
    }
}

#[component]
pub fn DialogStatus(message: ReadSignal<Option<String>>) -> impl IntoView {
    view! {
        {move || {
            message.get().map(|msg| view! {
                <div class="card-outer p-3 log-line-error">
                    <span class="tech-label">{msg}</span>
                </div>
            })
        }}
    }
}

#[component]
pub fn DialogShell<F, G>(
    title: F,
    subtitle: G,
    busy: ReadSignal<bool>,
    on_close: Box<dyn Fn() + Send + Sync>,
    children: Children,
) -> impl IntoView
where
    F: Fn() -> &'static str + Send + Sync + 'static,
    G: Fn() -> &'static str + Send + Sync + 'static,
{
    let on_close = StoredValue::new(on_close);
    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
            <div class="card-outer p-6 w-full max-w-xl mx-4 flex flex-col gap-4 max-h-[85vh] overflow-hidden">
                <div class="flex items-start justify-between border-b border-[var(--color-border)] pb-2">
                    <div class="space-y-2 min-w-0">
                        <span class="tech-label opacity-60">"config.dialog"</span>
                        <h2 class="text-lg font-bold break-words">{title()}</h2>
                        <p class="tech-label opacity-50 break-words">{subtitle()}</p>
                    </div>
                    <button
                        class="tech-label opacity-50 hover:opacity-100 px-2 py-0.5 border border-[var(--color-border)] cursor-pointer disabled:cursor-not-allowed disabled:opacity-20"
                        disabled=busy
                        on:click=move |_| on_close.with_value(|f| f())
                    >
                        "✕"
                    </button>
                </div>
                <div class="min-h-0 overflow-y-auto overflow-x-hidden">
                    {children()}
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn DialogActions(
    busy: ReadSignal<bool>,
    submit_label: &'static str,
    cancel: Box<dyn Fn() + Send + Sync>,
) -> impl IntoView {
    let cancel = StoredValue::new(cancel);
    view! {
        <div class="flex justify-end gap-2">
            <button
                type="button"
                class="btn"
                disabled=busy
                on:click=move |_| cancel.with_value(|f| f())
            >
                "CANCEL"
            </button>
            <button type="submit" class="btn btn-primary" disabled=busy>
                {move || if busy.get() { "SAVING..." } else { submit_label }}
            </button>
        </div>
    }
}
