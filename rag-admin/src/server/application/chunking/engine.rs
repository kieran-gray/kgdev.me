use std::{collections::BTreeMap, sync::Arc};

use crate::{
    server::application::{
        chunking::{ports::TextChunker, ChunkOutput},
        ports::Tokenizer,
        AppError,
    },
    shared::{ChunkStrategy, ChunkingConfig},
};

pub struct ChunkingEngine {
    chunkers: BTreeMap<ChunkStrategy, Arc<dyn TextChunker>>,
    tokenizer: Arc<dyn Tokenizer>,
}

impl ChunkingEngine {
    pub fn new(tokenizer: Arc<dyn Tokenizer>) -> Self {
        Self {
            chunkers: BTreeMap::new(),
            tokenizer,
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
        let chunker = self.chunkers.get(&config.strategy).ok_or_else(|| {
            AppError::Validation(format!(
                "unsupported chunking strategy '{}'",
                config.strategy.as_str()
            ))
        })?;

        let chunks = chunker
            .chunk(config, source, self.tokenizer.as_ref())
            .await?;
        Ok(chunks)
    }
}
