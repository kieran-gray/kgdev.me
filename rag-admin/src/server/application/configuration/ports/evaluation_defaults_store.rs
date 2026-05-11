use async_trait::async_trait;

use crate::server::application::AppError;
use crate::shared::SettingsDto;

/// Persistence for the evaluation-defaults form (question count, thresholds,
/// etc.). Kept as a separate port so the (forthcoming) UI overhaul can swap
/// the file-backed adapter for an event-sourced aggregate without touching
/// the call sites.
#[async_trait]
pub trait EvaluationDefaultsStore: Send + Sync {
    async fn load(&self) -> Result<SettingsDto, AppError>;
    async fn save(&self, settings: SettingsDto) -> Result<(), AppError>;
}
