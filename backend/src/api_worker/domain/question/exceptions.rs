use std::{
    error::Error,
    fmt::{Display, Formatter, Result},
};

#[derive(Debug, Clone)]
pub enum QuestionValidationError {
    TooShort,
    TooLong,
    InvalidFormat(String),
}

impl Display for QuestionValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            QuestionValidationError::TooShort => write!(f, "Question is too short"),
            QuestionValidationError::TooLong => write!(f, "Question is too long"),
            QuestionValidationError::InvalidFormat(msg) => write!(f, "Invalid format: {msg}"),
        }
    }
}

impl Error for QuestionValidationError {}
