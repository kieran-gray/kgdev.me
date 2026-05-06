use leptos::prelude::*;

use crate::shared::{LogEvent, LogLevel};

#[component]
pub fn LogPanel(events: ReadSignal<Vec<LogEvent>>) -> impl IntoView {
    view! {
        <div class="log-pre h-full bg-black/40">
            {move || {
                let events = events.get();
                if events.is_empty() {
                    view! { <span class="tech-label opacity-30">"AWAITING_SIGNAL... // NO_LOG_ENTRIES"</span> }
                        .into_any()
                } else {
                    events
                        .into_iter()
                        .map(|e| {
                            let (prefix, cls) = match e.level {
                                LogLevel::Info => ("INFO", "log-line-info"),
                                LogLevel::Warn => ("WARN", "log-line-warn"),
                                LogLevel::Error => ("ERRO", "log-line-error"),
                                LogLevel::Success => ("DONE", "log-line-success"),
                            };
                            view! {
                                <div class="flex gap-2">
                                    <span class=format!("tech-label opacity-40 shrink-0 {}", cls)>{prefix}</span>
                                    <span class=cls>{e.message}</span>
                                </div>
                            }
                        })
                        .collect_view()
                        .into_any()
                }
            }}
        </div>
    }
}
