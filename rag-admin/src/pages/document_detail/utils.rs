use crate::shared::LogEvent;
use leptos::prelude::*;

#[cfg(feature = "hydrate")]
pub fn open_event_stream(
    url: String,
    set_events: WriteSignal<Vec<LogEvent>>,
    set_running: WriteSignal<bool>,
) {
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::{EventSource, MessageEvent};
    use crate::shared::LogLevel;

    let source = match EventSource::new(&url) {
        Ok(s) => s,
        Err(e) => {
            set_events.update(|evs| {
                evs.push(LogEvent {
                    level: LogLevel::Error,
                    message: format!("failed to open event stream: {:?}", e),
                });
            });
            set_running.set(false);
            return;
        }
    };

    let source_for_close = source.clone();
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |evt: MessageEvent| {
        let data = evt.data().as_string().unwrap_or_default();
        if data == "__done__" {
            set_running.set(false);
            source_for_close.close();
            return;
        }
        match serde_json::from_str::<LogEvent>(&data) {
            Ok(e) => set_events.update(|evs| evs.push(e)),
            Err(err) => set_events.update(|evs| {
                evs.push(LogEvent {
                    level: LogLevel::Error,
                    message: format!("malformed log event: {err}"),
                });
            }),
        }
    });
    source.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();

    let source_for_err = source.clone();
    let on_error = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        set_running.set(false);
        source_for_err.close();
    });
    source.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    on_error.forget();
}

#[cfg(not(feature = "hydrate"))]
pub fn open_event_stream(
    _url: String,
    _set_events: WriteSignal<Vec<LogEvent>>,
    _set_running: WriteSignal<bool>,
) {
}

pub fn short_hash(hash: &str) -> &str {
    if hash.len() > 12 {
        &hash[..12]
    } else {
        hash
    }
}
