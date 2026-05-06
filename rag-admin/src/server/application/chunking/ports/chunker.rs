use async_trait::async_trait;

use crate::{
    server::{
        application::{markdown::Document, ports::{MarkdownParser, Tokenizer}, AppError},
        domain::Chunk,
    },
    shared::{ChunkStrategy, ChunkingConfig},
};

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

#[async_trait]
pub trait TextChunker: Send + Sync {
    fn strategy(&self) -> ChunkStrategy;

    async fn chunk(
        &self,
        config: ChunkingConfig,
        source: &str,
        tokenizer: &dyn Tokenizer,
    ) -> Result<Vec<ChunkOutput>, AppError>;
}

pub trait MarkdownBackedChunker {
    fn markdown_parser(&self) -> &dyn MarkdownParser;

    fn parse_markdown(&self, source: &str) -> Result<Document, AppError> {
        self.markdown_parser().parse(source)
    }
}
