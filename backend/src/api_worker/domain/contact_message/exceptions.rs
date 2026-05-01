#[derive(Debug, Clone)]
pub enum ContactMessageValidationError {
    InvalidEmail(String),
    InvalidName(String),
    InvalidMessage(String),
}

impl std::fmt::Display for ContactMessageValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContactMessageValidationError::InvalidEmail(msg) => write!(f, "Invalid email: {msg}"),
            ContactMessageValidationError::InvalidName(msg) => write!(f, "Invalid name: {msg}"),
            ContactMessageValidationError::InvalidMessage(msg) => {
                write!(f, "Invalid message: {msg}")
            }
        }
    }
}

impl std::error::Error for ContactMessageValidationError {}
