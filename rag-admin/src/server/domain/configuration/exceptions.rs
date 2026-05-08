#[derive(Debug, Clone)]
pub enum ConfigurationError {
    NotFound,
    ValidationError(String),
    InvalidCommand(String),
    InvalidEvent(String),
}

impl std::fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigurationError::NotFound => write!(f, "Configuration not found"),
            ConfigurationError::ValidationError(msg) => write!(f, "{msg}"),
            ConfigurationError::InvalidCommand(msg) => write!(f, "{msg}"),
            ConfigurationError::InvalidEvent(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ConfigurationError {}
