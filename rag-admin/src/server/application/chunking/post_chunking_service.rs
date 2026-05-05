use std::sync::Arc;

use crate::server::application::chunking::ChunkOutput;
use crate::server::application::chunking::ChunkingEngine;
use crate::server::application::AppError;
use crate::server::domain::{Chunk, Post};
use crate::shared::ChunkingConfig;

pub struct PostChunkingService {
    chunking_engine: Arc<ChunkingEngine>,
}

pub struct ChunkedPost {
    pub body_chunks: Vec<ChunkOutput>,
    pub glossary_chunks: Vec<Chunk>,
}

impl ChunkedPost {
    pub fn body_chunk_count(&self) -> u32 {
        self.body_chunks.len() as u32
    }

    pub fn glossary_chunk_count(&self) -> u32 {
        self.glossary_chunks.len() as u32
    }

    pub fn total_chunk_count(&self) -> u32 {
        self.body_chunk_count() + self.glossary_chunk_count()
    }

    pub fn into_chunks(self) -> Vec<Chunk> {
        self.body_chunks
            .into_iter()
            .map(Chunk::from)
            .chain(self.glossary_chunks)
            .collect()
    }
}

impl PostChunkingService {
    pub fn new(chunking_engine: Arc<ChunkingEngine>) -> Arc<Self> {
        Arc::new(Self { chunking_engine })
    }

    pub async fn chunk_post(
        &self,
        post: &Post,
        config: ChunkingConfig,
        include_glossary: bool,
    ) -> Result<ChunkedPost, AppError> {
        let body_chunks = self
            .chunking_engine
            .chunk_text(config, post.markdown_body())
            .await?;
        let glossary_chunks = if include_glossary {
            post.glossary_chunks(body_chunks.len() as u32)
        } else {
            Vec::new()
        };

        Ok(ChunkedPost {
            body_chunks,
            glossary_chunks,
        })
    }
}
