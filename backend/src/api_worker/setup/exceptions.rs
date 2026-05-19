use std::{
    error::Error,
    fmt::{Display, Formatter, Result},
};

#[derive(Debug)]
pub enum SetupError {
    MissingVariable(String),
    InvalidVariable(String),
    MissingBinding(String),
    DurableObject(String),
}

impl Display for SetupError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            SetupError::MissingVariable(msg) => write!(f, "Environment Variable Not Found: {msg}"),
            SetupError::InvalidVariable(msg) => write!(f, "Invalid Environment Variable: {msg}"),
            SetupError::MissingBinding(msg) => write!(f, "Binding Not Found: {msg}"),
            SetupError::DurableObject(msg) => write!(f, "Failure in durable object setup: {msg}"),
        }
    }
}

impl Error for SetupError {}
