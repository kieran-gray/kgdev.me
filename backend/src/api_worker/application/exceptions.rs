use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    Unauthorised(String),
    InternalError(String),
    ValidationError(String),
    RateLimited(String),
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NotFound(msg) => write!(f, "Not found: {msg}"),
            AppError::Unauthorised(msg) => write!(f, "Unauthorised: {msg}"),
            AppError::InternalError(msg) => write!(f, "Internal server error: {msg}"),
            AppError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
            AppError::RateLimited(msg) => write!(f, "Rate limited: {msg}"),
        }
    }
}

impl Error for AppError {}
