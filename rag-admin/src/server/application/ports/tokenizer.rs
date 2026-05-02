use crate::server::application::AppError;

#[derive(Debug, Clone)]
pub struct Tokenized {
    pub tokens: Vec<String>,
    pub count: u32,
}

pub trait Tokenizer: Send + Sync {
    fn encode(&self, text: &str) -> Result<Tokenized, AppError>;
}
