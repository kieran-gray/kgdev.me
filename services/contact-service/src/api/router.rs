use crate::{
    api::{cors::CorsContext, requests::CreateContactMessageRequest},
    application::AppError,
    setup::AppState,
};

use tracing::{error, info};
use worker::{Request, Response, RouteContext, Router};

pub fn create_router(app_state: AppState) -> Router<'static, AppState> {
    let router = Router::with_data(app_state);
    router
        .post_async("/api/v1/contact/", |req, ctx| async move {
            let cors_context = CorsContext::new(ctx.data.config.allowed_origins.clone(), &req);
            if let Err(response) = cors_context.validate(&req) {
                return Ok(response);
            }
            create_contact_message_handler(req, ctx, cors_context).await
        })
        .options("/api/v1/contact/", |req, ctx| {
            let cors_context = CorsContext::new(ctx.data.config.allowed_origins, &req);
            match cors_context.validate(&req) {
                Ok(_) => cors_context.preflight_response(),
                Err(response) => Ok(response),
            }
        })
}

async fn create_contact_message_handler(
    mut req: Request,
    ctx: RouteContext<AppState>,
    cors_context: CorsContext,
) -> worker::Result<Response> {
    let payload: CreateContactMessageRequest = match req.json().await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to parse request body: {:?}", e);
            let response = Response::from(AppError::ValidationError(
                "Failed to parse request body".into(),
            ));
            return Ok(cors_context.add_to_response(response));
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
            let response = Response::from_json(&serde_json::json!({ "success": true }))?;
            Ok(cors_context.add_to_response(response))
        }
        Err(e) => {
            error!("Failed to create message: {:?}", e);
            let response = Response::from(e);
            Ok(cors_context.add_to_response(response))
        }
    }
}
