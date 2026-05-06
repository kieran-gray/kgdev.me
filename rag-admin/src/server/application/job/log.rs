use serde::{Deserialize, Serialize};

use crate::shared::{LogEvent, LogLevel};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IngestLogLevel {
    Info,
    Warn,
    Error,
    Success,
}

impl From<IngestLogLevel> for LogLevel {
    fn from(value: IngestLogLevel) -> Self {
        match value {
            IngestLogLevel::Info => LogLevel::Info,
            IngestLogLevel::Warn => LogLevel::Warn,
            IngestLogLevel::Error => LogLevel::Error,
            IngestLogLevel::Success => LogLevel::Success,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestLogEvent {
    pub level: IngestLogLevel,
    pub message: String,
}

impl IngestLogEvent {
    pub fn info(msg: impl Into<String>) -> Self {
        Self {
            level: IngestLogLevel::Info,
            message: msg.into(),
        }
    }
    pub fn warn(msg: impl Into<String>) -> Self {
        Self {
            level: IngestLogLevel::Warn,
            message: msg.into(),
        }
    }
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            level: IngestLogLevel::Error,
            message: msg.into(),
        }
    }
    pub fn success(msg: impl Into<String>) -> Self {
        Self {
            level: IngestLogLevel::Success,
            message: msg.into(),
        }
    }
}

impl From<IngestLogEvent> for LogEvent {
    fn from(value: IngestLogEvent) -> Self {
        LogEvent {
            level: value.level.into(),
            message: value.message,
        }
    }
}
