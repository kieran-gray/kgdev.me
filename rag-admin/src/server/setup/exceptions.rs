use thiserror::Error;

#[derive(Debug, Error)]
pub enum SetupError {
    #[error("config error: {0}")]
    Config(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("internal error: {0}")]
    Internal(String),
}
