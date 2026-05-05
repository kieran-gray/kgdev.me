use std::{collections::BTreeMap, sync::Arc};

use crate::{
    server::application::{
        chunking::{ports::TextChunker, ChunkOutput},
        AppError,
    },
    shared::{ChunkStrategy, ChunkingConfig},
};

#[derive(Default)]
pub struct ChunkingEngine {
    chunkers: BTreeMap<ChunkStrategy, Arc<dyn TextChunker>>,
}

impl ChunkingEngine {
    pub fn new() -> Self {
        Self {
            chunkers: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, chunker: Arc<dyn TextChunker>) {
        self.chunkers.insert(chunker.strategy(), chunker);
    }

    pub async fn chunk_text(
        &self,
        config: ChunkingConfig,
        source: &str,
    ) -> Result<Vec<ChunkOutput>, AppError> {
        let chunker = self
            .chunkers
            .get(&config.strategy)
            .ok_or_else(|| AppError::Validation("unsupported chunking strategy".into()))?;

        let chunks = chunker.chunk(config, source).await?;
        Ok(chunks)
    }
}
