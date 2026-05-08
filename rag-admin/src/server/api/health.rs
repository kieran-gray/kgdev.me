use std::sync::Arc;

use crate::server::setup::AppState;
use axum::response::Response;
use axum::Extension;

pub async fn health_check(Extension(_state): Extension<Arc<AppState>>) -> Response {
    Response::new("ok".into())
}
