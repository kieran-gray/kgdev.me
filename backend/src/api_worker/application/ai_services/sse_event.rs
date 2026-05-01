use crate::api_worker::application::CachedSource;

#[derive(Debug, Clone)]
pub enum SseEvent {
    Meta {
        sources: Vec<CachedSource>,
        cached: bool,
        model: String,
    },
    Delta {
        text: String,
    },
    Done,
    Error {
        message: String,
    },
}

impl SseEvent {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Meta {
                sources,
                cached,
                model,
            } => {
                let payload = serde_json::json!({
                    "sources": sources,
                    "cached": cached,
                    "model": model,
                });
                format!("event: meta\ndata: {payload}\n\n").into_bytes()
            }
            Self::Delta { text } => {
                let payload = serde_json::json!({ "text": text });
                format!("event: delta\ndata: {payload}\n\n").into_bytes()
            }
            Self::Done => b"event: done\ndata: {}\n\n".to_vec(),
            Self::Error { message } => {
                let payload = serde_json::json!({ "message": message });
                format!("event: error\ndata: {payload}\n\n").into_bytes()
            }
        }
    }
}
