#[derive(Debug, Clone)]
pub enum ValidationError {
    InvalidEmail(String),
    InvalidName(String),
    InvalidMessage(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidEmail(msg) => write!(f, "Invalid email: {msg}"),
            ValidationError::InvalidName(msg) => write!(f, "Invalid name: {msg}"),
            ValidationError::InvalidMessage(msg) => write!(f, "Invalid message: {msg}"),
        }
    }
}

impl std::error::Error for ValidationError {}
