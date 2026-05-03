use crate::shared::{ChunkStrategy, ChunkingConfig};

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

pub fn chunk(config: ChunkingConfig, source: &str) -> Vec<ChunkOutput> {
    match config.strategy {
        ChunkStrategy::Bert => bert::chunk(config, source),
        ChunkStrategy::Section => section::chunk(config, source),
    }
}
