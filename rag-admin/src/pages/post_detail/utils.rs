use crate::shared::{ChunkingConfig, ChunkingVariant, LogEvent};
use leptos::prelude::*;

#[cfg(feature = "hydrate")]
use crate::shared::LogLevel;

#[cfg(feature = "hydrate")]
pub fn open_event_stream(
    url: String,
    set_events: WriteSignal<Vec<LogEvent>>,
    set_running: WriteSignal<bool>,
) {
    open_event_stream_with_done(url, set_events, set_running, || {});
}

#[cfg(feature = "hydrate")]
pub fn open_event_stream_with_done<F>(
    url: String,
    set_events: WriteSignal<Vec<LogEvent>>,
    set_running: WriteSignal<bool>,
    on_done: F,
) where
    F: Fn() + 'static,
{
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::{EventSource, MessageEvent};

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
    let set_events_msg = set_events;
    let set_running_msg = set_running;
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |evt: MessageEvent| {
        let data = evt.data().as_string().unwrap_or_default();
        if data == "__done__" {
            set_running_msg.set(false);
            source_for_close.close();
            on_done();
            return;
        }
        match serde_json::from_str::<LogEvent>(&data) {
            Ok(e) => set_events_msg.update(|evs| evs.push(e)),
            Err(err) => set_events_msg.update(|evs| {
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

#[cfg(not(feature = "hydrate"))]
pub fn open_event_stream_with_done<F>(
    _url: String,
    _set_events: WriteSignal<Vec<LogEvent>>,
    _set_running: WriteSignal<bool>,
    _on_done: F,
) where
    F: Fn() + 'static,
{
}

pub fn short_hash(hash: &str) -> String {
    if hash.len() <= 12 {
        hash.to_string()
    } else {
        format!("{}...", &hash[..12])
    }
}

pub fn sweep_variants(current: ChunkingConfig) -> Vec<ChunkingVariant> {
    ChunkingConfig::sweep_configs(current)
        .into_iter()
        .map(|config| ChunkingVariant {
            label: chunking_variant_label(&config),
            config,
        })
        .collect()
}

pub fn chunking_variant_label(config: &ChunkingConfig) -> String {
    config.display_label()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ChunkStrategy;

    fn default_config(strategy: ChunkStrategy) -> ChunkingConfig {
        ChunkingConfig {
            strategy,
            ..ChunkingConfig::default()
        }
    }

    #[test]
    fn sweep_variants_includes_llm_candidates() {
        let variants = sweep_variants(default_config(ChunkStrategy::Section));

        let labels = variants
            .iter()
            .map(|variant| variant.label.as_str())
            .collect::<Vec<_>>();

        assert!(labels.contains(&"llm:64"));
        assert!(labels.contains(&"llm:96"));
        assert!(labels.contains(&"llm:128"));
        assert!(variants
            .iter()
            .any(|variant| variant.config.strategy == ChunkStrategy::Llm));
    }

    #[test]
    fn sweep_variants_keeps_current_llm_without_duplicate() {
        let current = ChunkingConfig {
            strategy: ChunkStrategy::Llm,
            llm_micro_chunk_tokens: 96,
            ..ChunkingConfig::default()
        };

        let variants = sweep_variants(current);

        assert_eq!(variants.first().unwrap().label, "llm:96");
        assert_eq!(
            variants
                .iter()
                .filter(|variant| variant.config == current)
                .count(),
            1
        );
    }
}

pub fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();

    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}
