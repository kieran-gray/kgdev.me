use crate::server::application::ports::{Tokenized, Tokenizer};
use crate::server::application::AppError;

/// Whitespace-splitting tokenizer for unit tests that need a `Tokenizer` but
/// don't care about real subword behaviour. The chunker tests use this so they
/// don't pull a full HuggingFace tokenizer into the test binary.
pub struct MockTokenizer;

impl MockTokenizer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tokenizer for MockTokenizer {
    fn encode(&self, text: &str) -> Result<Tokenized, AppError> {
        let tokens: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
        let count = tokens.len() as u32;
        Ok(Tokenized { tokens, count })
    }
}
