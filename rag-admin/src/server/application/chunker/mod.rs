use crate::{server::domain::Chunk, shared::{ChunkStrategy, ChunkingConfig}};

mod bert;
mod common;
mod section;

#[derive(Debug, Clone)]
pub struct ChunkOutput {
    pub chunk_id: u32,
    pub heading: String,
    pub text: String,
    pub char_start: u32,
    pub char_end: u32,
}

impl From<ChunkOutput> for Chunk {
    fn from(value: ChunkOutput) -> Chunk {
        Chunk {
            chunk_id: value.chunk_id,
            heading: value.heading,
            text: value.text,
            char_start: value.char_start,
            char_end: value.char_end,
            sources: Vec::new(),
            is_glossary: false,
        }
    }
}

pub fn chunk(config: ChunkingConfig, source: &str) -> Vec<ChunkOutput> {
    match config.strategy {
        ChunkStrategy::Bert => bert::chunk(config, source),
        ChunkStrategy::Section => section::chunk(config, source),
    }
}
