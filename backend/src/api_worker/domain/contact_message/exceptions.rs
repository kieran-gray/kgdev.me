use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Debug, Clone)]
pub enum ContactMessageValidationError {
    InvalidEmail(String),
    InvalidName(String),
    InvalidMessage(String),
}

impl Display for ContactMessageValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ContactMessageValidationError::InvalidEmail(msg) => write!(f, "Invalid email: {msg}"),
            ContactMessageValidationError::InvalidName(msg) => write!(f, "Invalid name: {msg}"),
            ContactMessageValidationError::InvalidMessage(msg) => {
                write!(f, "Invalid message: {msg}")
            }
        }
    }
}

impl Error for ContactMessageValidationError {}
