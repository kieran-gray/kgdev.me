use crate::api_worker::{
    AppState, api::schema::requests::CreateContactMessageRequest, application::AppError,
};
use tracing::{error, info};
use worker::{Request, Response, RouteContext};

pub async fn create_contact_message_handler(
    mut req: Request,
    ctx: RouteContext<AppState>,
) -> worker::Result<Response> {
    let payload: CreateContactMessageRequest = match req.json().await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to parse request body: {:?}", e);
            return Ok(Response::from(AppError::ValidationError(
                "Failed to parse request body".into(),
            )));
        }
    };

    let ip_address = req
        .headers()
        .get("CF-Connecting-IP")
        .ok()
        .flatten()
        .unwrap_or_else(|| "0.0.0.0".to_string());

    match ctx
        .data
        .contact_message_service
        .new_message(
            payload.token,
            ip_address,
            payload.email,
            payload.name,
            payload.message,
        )
        .await
    {
        Ok(_) => {
            info!("Contact message created successfully.");
            Response::from_json(&serde_json::json!({ "success": true }))
        }
        Err(e) => {
            error!("Failed to create message: {:?}", e);
            Ok(Response::from(e))
        }
    }
}
