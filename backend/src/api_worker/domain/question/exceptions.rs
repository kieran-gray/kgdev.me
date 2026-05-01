#[derive(Debug, Clone)]
pub enum QuestionValidationError {
    TooShort,
    TooLong,
    InvalidFormat(String),
}

impl std::fmt::Display for QuestionValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuestionValidationError::TooShort => write!(f, "Question is too short"),
            QuestionValidationError::TooLong => write!(f, "Question is too long"),
            QuestionValidationError::InvalidFormat(msg) => write!(f, "Invalid format: {msg}"),
        }
    }
}

impl std::error::Error for QuestionValidationError {}
