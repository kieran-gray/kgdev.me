use tracing::error;
use worker::{Request, Response, RouteContext};

use crate::api_worker::{AppState, application::AppError, domain::PostSlug};

pub async fn handle_websocket_connect(
    req: Request,
    ctx: RouteContext<AppState>,
) -> worker::Result<Response> {
    let slug = match ctx.param("page") {
        Some(p) => match PostSlug::parse(p) {
            Ok(slug) => slug,
            Err(e) => return Ok(Response::from(AppError::ValidationError(e.to_string()))),
        },
        None => {
            let error = "No page provided";
            error!(error);
            return Response::error(error, 400);
        }
    };

    if !ctx.data.config.security.allowed_blog_paths.contains(&slug) {
        let error = format!("Path not allowed: {slug}");
        error!(error);
        return Response::error(error, 403);
    }

    match req.headers().get("Upgrade") {
        Ok(Some(h)) if h == "websocket" => (),
        _ => return Ok(Response::empty()?.with_status(426)),
    }

    let response = ctx
        .data
        .view_counter_do_client
        .websocket_upgrade(slug.as_str())
        .await
        .map_err(|e| format!("Failed to connect to durable object: {e}"))?;

    Ok(response)
}
