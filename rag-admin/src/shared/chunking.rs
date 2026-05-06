use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum ChunkStrategy {
    Bert,
    #[default]
    Section,
    Llm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkParamKey {
    MaxSectionTokens,
    TargetTokens,
    OverlapTokens,
    MinTokens,
    LlmMicroChunkTokens,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkParamDefinition {
    pub key: ChunkParamKey,
    pub label: &'static str,
    pub hint: &'static str,
    pub min: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkerDefinition {
    pub strategy: ChunkStrategy,
    pub id: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub params: &'static [ChunkParamDefinition],
}

const SECTION_PARAMS: &[ChunkParamDefinition] = &[ChunkParamDefinition {
    key: ChunkParamKey::MaxSectionTokens,
    label: "MAX_SECTION_TOKENS",
    hint: "section: max tokens per chunk before fallback split",
    min: 1,
}];

const BERT_PARAMS: &[ChunkParamDefinition] = &[
    ChunkParamDefinition {
        key: ChunkParamKey::TargetTokens,
        label: "TARGET_TOKENS",
        hint: "bert: target chunk size in tokens",
        min: 1,
    },
    ChunkParamDefinition {
        key: ChunkParamKey::OverlapTokens,
        label: "OVERLAP_TOKENS",
        hint: "bert: tokens of overlap between adjacent chunks",
        min: 0,
    },
    ChunkParamDefinition {
        key: ChunkParamKey::MinTokens,
        label: "MIN_TOKENS",
        hint: "bert: small trailing chunks merge with the previous one",
        min: 0,
    },
];

const LLM_PARAMS: &[ChunkParamDefinition] = &[
    ChunkParamDefinition {
        key: ChunkParamKey::TargetTokens,
        label: "TARGET_TOKENS",
        hint: "llm: maximum final chunk size in tokens",
        min: 1,
    },
    ChunkParamDefinition {
        key: ChunkParamKey::LlmMicroChunkTokens,
        label: "MICRO_CHUNK_TOKENS",
        hint: "llm: punctuation-aware micro chunks offered to the model for boundary selection",
        min: 32,
    },
];

const CHUNKER_DEFINITIONS: &[ChunkerDefinition] = &[
    ChunkerDefinition {
        strategy: ChunkStrategy::Bert,
        id: "bert",
        label: "bert",
        hint: "sliding window with overlap",
        params: BERT_PARAMS,
    },
    ChunkerDefinition {
        strategy: ChunkStrategy::Section,
        id: "section",
        label: "section",
        hint: "heading-aware markdown sections",
        params: SECTION_PARAMS,
    },
    ChunkerDefinition {
        strategy: ChunkStrategy::Llm,
        id: "llm",
        label: "llm",
        hint: "LLM-selected semantic boundaries over micro chunks",
        params: LLM_PARAMS,
    },
];

impl ChunkStrategy {
    pub fn all() -> &'static [ChunkerDefinition] {
        CHUNKER_DEFINITIONS
    }

    pub fn as_str(self) -> &'static str {
        self.definition().id
    }

    pub fn from_id(value: &str) -> Option<Self> {
        CHUNKER_DEFINITIONS
            .iter()
            .find(|definition| definition.id == value)
            .map(|definition| definition.strategy)
    }

    pub fn definition(self) -> &'static ChunkerDefinition {
        CHUNKER_DEFINITIONS
            .iter()
            .find(|definition| definition.strategy == self)
            .expect("chunk strategy has a definition")
    }

    pub fn preview_limit_uses_tokens(self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkingConfig {
    pub strategy: ChunkStrategy,
    #[serde(deserialize_with = "crate::shared::serde_compat::u32_from_string")]
    pub max_section_tokens: u32,
    #[serde(deserialize_with = "crate::shared::serde_compat::u32_from_string")]
    pub target_tokens: u32,
    #[serde(deserialize_with = "crate::shared::serde_compat::u32_from_string")]
    pub overlap_tokens: u32,
    #[serde(deserialize_with = "crate::shared::serde_compat::u32_from_string")]
    pub min_tokens: u32,
    #[serde(
        default = "default_llm_micro_chunk_tokens",
        deserialize_with = "crate::shared::serde_compat::u32_from_string"
    )]
    pub llm_micro_chunk_tokens: u32,
}

fn default_llm_micro_chunk_tokens() -> u32 {
    96
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            strategy: ChunkStrategy::default(),
            max_section_tokens: 480,
            target_tokens: 384,
            overlap_tokens: 64,
            min_tokens: 96,
            llm_micro_chunk_tokens: default_llm_micro_chunk_tokens(),
        }
    }
}

impl ChunkingConfig {
    pub fn for_strategy(strategy: ChunkStrategy) -> Self {
        Self {
            strategy,
            ..Self::default()
        }
    }

    pub fn size_limit_for_display(&self, embedding_token_limit: u32) -> u32 {
        match self.strategy {
            ChunkStrategy::Bert | ChunkStrategy::Section | ChunkStrategy::Llm => {
                embedding_token_limit
            }
        }
    }

    pub fn max_section_tokens(&self) -> usize {
        self.max_section_tokens.max(1) as usize
    }

    pub fn param_value(&self, key: ChunkParamKey) -> u32 {
        match key {
            ChunkParamKey::MaxSectionTokens => self.max_section_tokens,
            ChunkParamKey::TargetTokens => self.target_tokens,
            ChunkParamKey::OverlapTokens => self.overlap_tokens,
            ChunkParamKey::MinTokens => self.min_tokens,
            ChunkParamKey::LlmMicroChunkTokens => self.llm_micro_chunk_tokens,
        }
    }

    pub fn set_param_value(&mut self, key: ChunkParamKey, value: u32) {
        match key {
            ChunkParamKey::MaxSectionTokens => self.max_section_tokens = value,
            ChunkParamKey::TargetTokens => self.target_tokens = value,
            ChunkParamKey::OverlapTokens => self.overlap_tokens = value,
            ChunkParamKey::MinTokens => self.min_tokens = value,
            ChunkParamKey::LlmMicroChunkTokens => self.llm_micro_chunk_tokens = value.max(32),
        }
    }

    pub fn display_label(&self) -> String {
        match self.strategy {
            ChunkStrategy::Section => format!("section:{}", self.max_section_tokens),
            ChunkStrategy::Bert => format!("bert:{}/{}", self.target_tokens, self.overlap_tokens),
            ChunkStrategy::Llm => format!("llm:{}", self.llm_micro_chunk_tokens),
        }
    }

    pub fn describe(&self) -> String {
        match self.strategy {
            ChunkStrategy::Bert => format!(
                "bert · target={} · overlap={} · min={}",
                self.target_tokens, self.overlap_tokens, self.min_tokens
            ),
            ChunkStrategy::Section => format!("section · max_tokens={}", self.max_section_tokens),
            ChunkStrategy::Llm => {
                format!("llm · micro_chunk_tokens={}", self.llm_micro_chunk_tokens)
            }
        }
    }

    pub fn detail_label(&self, size_limit: u32) -> String {
        match self.strategy {
            ChunkStrategy::Bert => format!(
                "STRATEGY: BERT · TOKEN_LIMIT: {} · TARGET: {} · OVERLAP: {} · MIN: {}",
                size_limit, self.target_tokens, self.overlap_tokens, self.min_tokens
            ),
            ChunkStrategy::Llm => format!(
                "STRATEGY: LLM · TOKEN_LIMIT: {} · TARGET: {} · MICRO_CHUNK_TOKENS: {}",
                size_limit, self.target_tokens, self.llm_micro_chunk_tokens
            ),
            ChunkStrategy::Section => {
                format!(
                    "STRATEGY: SECTION · MAX_TOKENS: {}",
                    self.max_section_tokens
                )
            }
        }
    }

    pub fn sweep_configs(current: Self) -> Vec<Self> {
        let mut configs = vec![current];

        for max_section_tokens in [256, 384, 480, 512] {
            push_unique_config(
                &mut configs,
                ChunkingConfig {
                    strategy: ChunkStrategy::Section,
                    max_section_tokens,
                    ..ChunkingConfig::default()
                },
            );
        }

        for (target_tokens, overlap_tokens) in [(256, 0), (320, 48), (384, 64), (448, 64)] {
            push_unique_config(
                &mut configs,
                ChunkingConfig {
                    strategy: ChunkStrategy::Bert,
                    target_tokens,
                    overlap_tokens,
                    min_tokens: 96,
                    ..ChunkingConfig::default()
                },
            );
        }

        for llm_micro_chunk_tokens in [64, 96, 128] {
            push_unique_config(
                &mut configs,
                ChunkingConfig {
                    strategy: ChunkStrategy::Llm,
                    llm_micro_chunk_tokens,
                    ..ChunkingConfig::default()
                },
            );
        }

        configs
    }
}

fn push_unique_config(configs: &mut Vec<ChunkingConfig>, config: ChunkingConfig) {
    if configs.iter().all(|existing| *existing != config) {
        configs.push(config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_strategy_definition_round_trips_by_id() {
        for definition in ChunkStrategy::all() {
            assert_eq!(
                ChunkStrategy::from_id(definition.id),
                Some(definition.strategy)
            );
            assert_eq!(definition.strategy.as_str(), definition.id);
        }
    }

    #[test]
    fn every_strategy_has_editable_params() {
        for definition in ChunkStrategy::all() {
            assert!(
                !definition.params.is_empty(),
                "{} has no params",
                definition.id
            );
        }
    }

    #[test]
    fn sweep_configs_are_unique_and_start_with_current() {
        let current = ChunkingConfig {
            strategy: ChunkStrategy::Llm,
            llm_micro_chunk_tokens: 96,
            ..ChunkingConfig::default()
        };
        let configs = ChunkingConfig::sweep_configs(current);

        assert_eq!(configs.first(), Some(&current));
        for (idx, config) in configs.iter().enumerate() {
            assert_eq!(
                configs
                    .iter()
                    .filter(|candidate| *candidate == config)
                    .count(),
                1,
                "duplicate sweep config at index {idx}"
            );
        }
    }
}
