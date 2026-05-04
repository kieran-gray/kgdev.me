use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ChunkStrategy {
    Bert,
    #[default]
    Section,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkingConfig {
    pub strategy: ChunkStrategy,
    pub max_section_chars: u32,
    pub target_chars: u32,
    pub overlap_chars: u32,
    pub min_chars: u32,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            strategy: ChunkStrategy::default(),
            max_section_chars: 8000,
            target_chars: 1600,
            overlap_chars: 240,
            min_chars: 320,
        }
    }
}

impl ChunkingConfig {
    pub fn size_limit_for_display(&self, embedding_token_limit: u32) -> u32 {
        match self.strategy {
            ChunkStrategy::Bert => embedding_token_limit,
            ChunkStrategy::Section => self.max_section_chars,
        }
    }

    pub fn max_section_chars(&self) -> usize {
        self.max_section_chars.max(1) as usize
    }
}
