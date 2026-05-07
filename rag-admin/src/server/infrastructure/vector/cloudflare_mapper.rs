use serde_json::{json, Value};

use crate::server::application::ingest::ports::VectorRecordMapper;
use crate::server::domain::{Chunk, Post, VectorRecord};

pub struct CloudflareVectorRecordMapper;

impl VectorRecordMapper for CloudflareVectorRecordMapper {
    fn map(&self, post: &Post, chunk: &Chunk, values: Vec<f32>) -> VectorRecord {
        VectorRecord {
            id: self.map_id(post, chunk),
            values,
            metadata: self.metadata_for(post, chunk),
        }
    }

    fn map_id(&self, post: &Post, chunk: &Chunk) -> String {
        format!("{}:{}", post.slug(), chunk.chunk_id)
    }
}

impl CloudflareVectorRecordMapper {
    fn metadata_for(&self, post: &Post, chunk: &Chunk) -> Value {
        let mut m = json!({
            "post_slug": post.slug(),
            "post_version": post.version().as_str(),
            "post_title": post.title(),
            "chunk_id": chunk.chunk_id,
            "heading": chunk.heading,
            "text": chunk.text,
            "char_start": chunk.char_start,
            "char_end": chunk.char_end,
        });
        if !chunk.sources.is_empty() {
            m["sources"] = Value::String(
                serde_json::to_string(&chunk.sources).unwrap_or_else(|_| "[]".to_string()),
            );
        }
        m
    }
}
