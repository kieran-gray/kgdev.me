pub mod state;
pub mod storage;

use std::cell::RefCell;

use serde::{Deserialize, Serialize};
use tracing::{error, warn};
use worker::{DurableObject, Env, Request, Response, Result, SqlStorage, State, durable_object};

use crate::blog_post_qa::state::TokenBucket;
use crate::blog_post_qa::storage::{check_and_increment_daily, init_schema, record_hit};

const BUCKET_CAPACITY: f64 = 6.0;
const BUCKET_REFILL_PER_HOUR: f64 = 30.0;

#[derive(Deserialize)]
struct ChargeBody {
    daily_cap: u32,
}

#[derive(Deserialize)]
struct HashBody {
    hash: String,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ChargeResponse {
    Ok,
    RateLimited { retry_after_ms: u64 },
    DailyCapExceeded,
}

#[durable_object]
pub struct BlogPostQA {
    state: State,
    bucket: RefCell<TokenBucket>,
    schema_ready: RefCell<bool>,
}

impl BlogPostQA {
    fn sql(&self) -> SqlStorage {
        self.state.storage().sql()
    }

    fn ensure_schema(&self) -> Result<()> {
        if *self.schema_ready.borrow() {
            return Ok(());
        }
        init_schema(&self.sql())?;
        *self.schema_ready.borrow_mut() = true;
        Ok(())
    }

    fn handle_charge(&self, body: ChargeBody) -> Result<Response> {
        let now = now_ms();

        if let Err(retry_after_ms) = self.bucket.borrow_mut().try_take(now) {
            return Response::from_json(&ChargeResponse::RateLimited { retry_after_ms });
        }

        self.ensure_schema()?;
        let date = utc_date(now);
        let allowed = check_and_increment_daily(&self.sql(), &date, body.daily_cap)?;
        if !allowed {
            return Response::from_json(&ChargeResponse::DailyCapExceeded);
        }

        Response::from_json(&ChargeResponse::Ok)
    }

    fn handle_record_hit(&self, body: HashBody) -> Result<Response> {
        self.ensure_schema()?;
        if let Err(e) = record_hit(&self.sql(), &body.hash, now_ms()) {
            warn!(error = %e, "qa_stats write failed");
        }
        Response::empty()
    }
}

impl DurableObject for BlogPostQA {
    fn new(state: State, _env: Env) -> Self {
        let now = now_ms();
        Self {
            state,
            bucket: RefCell::new(TokenBucket::new(
                BUCKET_CAPACITY,
                BUCKET_REFILL_PER_HOUR,
                now,
            )),
            schema_ready: RefCell::new(false),
        }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        let path = req.path();
        match path.as_str() {
            "/charge" => match req.json::<ChargeBody>().await {
                Ok(body) => self.handle_charge(body),
                Err(e) => {
                    error!(error = %e, "charge body parse failed");
                    Response::error("Bad Request", 400)
                }
            },
            "/record-hit" => match req.json::<HashBody>().await {
                Ok(body) => self.handle_record_hit(body),
                Err(e) => {
                    error!(error = %e, "record-hit body parse failed");
                    Response::error("Bad Request", 400)
                }
            },
            _ => Response::error("Not Found", 404),
        }
    }

    async fn alarm(&self) -> Result<Response> {
        Response::empty()
    }
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn utc_date(now_ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(now_ms)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string())
}
