use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum ChunkStrategy {
    Bert,
    #[default]
    Section,
    Llm,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkingConfig {
    pub strategy: ChunkStrategy,
    #[serde(deserialize_with = "crate::shared::serde_compat::u32_from_string")]
    pub max_section_chars: u32,
    #[serde(deserialize_with = "crate::shared::serde_compat::u32_from_string")]
    pub target_chars: u32,
    #[serde(deserialize_with = "crate::shared::serde_compat::u32_from_string")]
    pub overlap_chars: u32,
    #[serde(deserialize_with = "crate::shared::serde_compat::u32_from_string")]
    pub min_chars: u32,
    #[serde(
        default = "default_llm_micro_chunk_chars",
        deserialize_with = "crate::shared::serde_compat::u32_from_string"
    )]
    pub llm_micro_chunk_chars: u32,
}

fn default_llm_micro_chunk_chars() -> u32 {
    300
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            strategy: ChunkStrategy::default(),
            max_section_chars: 8000,
            target_chars: 1600,
            overlap_chars: 240,
            min_chars: 320,
            llm_micro_chunk_chars: default_llm_micro_chunk_chars(),
        }
    }
}

impl ChunkingConfig {
    pub fn size_limit_for_display(&self, embedding_token_limit: u32) -> u32 {
        match self.strategy {
            ChunkStrategy::Bert | ChunkStrategy::Llm => embedding_token_limit,
            ChunkStrategy::Section => self.max_section_chars,
        }
    }

    pub fn max_section_chars(&self) -> usize {
        self.max_section_chars.max(1) as usize
    }
}
