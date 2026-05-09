#[derive(Debug, Clone)]
pub enum SourceDocumentError {
    NotFound,
    ValidationError(String),
    InvalidCommand(String),
    InvalidEvent(String),
}

impl std::fmt::Display for SourceDocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceDocumentError::NotFound => write!(f, "Source Document not found"),
            SourceDocumentError::ValidationError(msg) => write!(f, "{msg}"),
            SourceDocumentError::InvalidCommand(msg) => write!(f, "{msg}"),
            SourceDocumentError::InvalidEvent(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for SourceDocumentError {}