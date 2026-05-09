use crate::shared::ChunkDto;
use leptos::prelude::*;

#[component]
pub fn ChunkCard(chunk: ChunkDto) -> impl IntoView {
    let prefix = "DOCUMENT_CHUNK";
    let text_length = chunk.text.len();
    let heading = chunk.heading.clone();
    let sequence = chunk.sequence;

    let length_label = format!("TEXT_LENGTH: {text_length} CHARS");
    let seq_label = format!("SEQ: {:03}", sequence);

    let text = StoredValue::new(chunk.text);

    view! {
        <div class="card-inner p-3 relative overflow-hidden group">
            <div class="flex flex-row justify-between">
                <div class="flex flex-col mb-2">
                    <span class="tech-label text-[var(--color-accent)]">{prefix}</span>
                    <span class="font-bold text-sm uppercase tracking-tight">{heading}</span>
                </div>
                <div class="flex flex-col items-end">
                    <span class="tech-label opacity-40">{seq_label}</span>
                </div>
            </div>
            <div>
                <pre class="log-pre text-[10px] bg-transparent border-none p-0 max-h-[10rem] whitespace-pre-wrap">
                    {text.get_value()}
                </pre>
            </div>
            <div class="mt-2 flex justify-between items-center tech-label">
                <span class="opacity-40">{length_label}</span>
            </div>
            <div class="mt-2 pt-2 border-t border-[var(--color-border)] flex justify-end">
                <a
                    href=text.with_value(|t| format!("/embed?a={}", urlencoding::encode(t)))
                    class="tech-label opacity-40 hover:opacity-100 transition-opacity"
                >
                    "PROBE_EMBED →"
                </a>
            </div>
        </div>
    }
}
