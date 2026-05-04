use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tracing::error;

use crate::api_worker::{
    application::{AppError, ChargeOutcome, QaCoordinatorTrait},
    infrastructure::durable_object_client::DurableObjectClient,
};

pub struct QaCoordinatorDoService {
    client: DurableObjectClient,
}

impl QaCoordinatorDoService {
    pub fn create(client: DurableObjectClient) -> Arc<Self> {
        Arc::new(Self { client })
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ChargeDto {
    Ok,
    RateLimited { retry_after_ms: u64 },
    DailyCapExceeded,
}

impl From<ChargeDto> for ChargeOutcome {
    fn from(dto: ChargeDto) -> Self {
        match dto {
            ChargeDto::Ok => ChargeOutcome::Ok,
            ChargeDto::RateLimited { retry_after_ms } => {
                ChargeOutcome::RateLimited { retry_after_ms }
            }
            ChargeDto::DailyCapExceeded => ChargeOutcome::DailyCapExceeded,
        }
    }
}

#[async_trait(?Send)]
impl QaCoordinatorTrait for QaCoordinatorDoService {
    async fn charge(&self, slug: &str, daily_cap: u32) -> Result<ChargeOutcome, AppError> {
        let mut response = self
            .client
            .post(slug, "/charge", json!({ "daily_cap": daily_cap }))
            .await
            .map_err(|err| AppError::InternalError(err.to_string()))?;

        let dto: ChargeDto = response.json().await.map_err(|e| {
            error!(error = %e, "qa charge parse failed");
            AppError::InternalError("QA coordinator response invalid".to_string())
        })?;
        Ok(dto.into())
    }

    async fn record_hit(&self, slug: &str, hash: &str) -> Result<(), AppError> {
        self.client
            .post(slug, "/record-hit", json!({ "hash": hash }))
            .await
            .map_err(|err| AppError::InternalError(err.to_string()))?;
        Ok(())
    }
}
