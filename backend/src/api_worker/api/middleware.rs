use worker::{Request, Response, Result, RouteContext};

use crate::api_worker::AppState;
use crate::api_worker::api::cors::CorsContext;

pub async fn public<F, Fut>(
    handler: F,
    req: Request,
    ctx: RouteContext<AppState>,
) -> Result<Response>
where
    F: Fn(Request, RouteContext<AppState>) -> Fut,
    Fut: std::future::Future<Output = Result<Response>>,
{
    let cors_context = CorsContext::new(ctx.data.config.allowed_origins.clone(), &req);
    if let Err(response) = cors_context.validate(&req) {
        return Ok(response);
    }
    let result = handler(req, ctx).await?;
    Ok(cors_context.add_to_response(result))
}

pub fn create_options_handler(
    req: Request,
    ctx: RouteContext<AppState>,
) -> worker::Result<Response> {
    let cors_context = CorsContext::new(ctx.data.config.allowed_origins, &req);
    match cors_context.validate(&req) {
        Ok(_) => cors_context.preflight_response(),
        Err(response) => Ok(response),
    }
}
