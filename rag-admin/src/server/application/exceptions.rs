use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::server::domain::{
    configuration::exceptions::ConfigurationError,
    pipeline_configuration::PipelineConfigurationRepositoryError,
};

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum AppError {
    #[error("domain error: {0}")]
    Domain(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<ConfigurationError> for AppError {
    fn from(value: ConfigurationError) -> Self {
        AppError::Domain(value.to_string())
    }
}

impl From<PipelineConfigurationRepositoryError> for AppError {
    fn from(value: PipelineConfigurationRepositoryError) -> Self {
        AppError::Internal(value.to_string())
    }
}
