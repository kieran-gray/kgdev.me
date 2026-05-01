use async_trait::async_trait;

use crate::api_worker::application::AppError;

#[derive(Debug, Clone)]
pub enum ChargeOutcome {
    Ok,
    RateLimited { retry_after_ms: u64 },
    DailyCapExceeded,
}

#[async_trait(?Send)]
pub trait QaCoordinatorTrait {
    async fn charge(&self, slug: &str, daily_cap: u32) -> Result<ChargeOutcome, AppError>;
    async fn record_hit(&self, slug: &str, hash: &str) -> Result<(), AppError>;
}
