use crate::application::exceptions::AppError;
use serde_json::json;

const HTTP_STATUS_BAD_REQUEST: u16 = 400;
const HTTP_STATUS_UNAUTHORISED: u16 = 401;
const HTTP_STATUS_NOT_FOUND: u16 = 404;
const HTTP_STATUS_INTERNAL_ERROR: u16 = 500;

impl From<AppError> for worker::Response {
    fn from(error: AppError) -> Self {
        let (status, message) = match &error {
            AppError::NotFound(msg) => (HTTP_STATUS_NOT_FOUND, msg.as_str()),
            AppError::Unauthorised(msg) => (HTTP_STATUS_UNAUTHORISED, msg.as_str()),
            AppError::InternalError(_) => (HTTP_STATUS_INTERNAL_ERROR, "Internal server error"),
            AppError::ValidationError(msg) => (HTTP_STATUS_BAD_REQUEST, msg.as_str()),
        };

        let body = json!({ "success": false, "error": message });
        worker::Response::from_json(&body)
            .unwrap()
            .with_status(status)
    }
}
