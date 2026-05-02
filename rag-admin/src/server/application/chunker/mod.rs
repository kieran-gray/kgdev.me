use crate::shared::ChunkStrategy;

mod bert;
mod common;
mod section;

pub use section::MAX_SECTION_CHARS;

#[derive(Debug, Clone)]
pub struct ChunkOutput {
    pub chunk_id: u32,
    pub heading: String,
    pub text: String,
    pub char_start: u32,
    pub char_end: u32,
}

pub fn chunk(strategy: ChunkStrategy, source: &str) -> Vec<ChunkOutput> {
    match strategy {
        ChunkStrategy::Bert => bert::chunk(source),
        ChunkStrategy::Section => section::chunk(source),
    }
}
