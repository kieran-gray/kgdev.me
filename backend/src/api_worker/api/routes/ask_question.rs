use futures_util::StreamExt;
use tracing::{error, info};
use worker::{Request, Response, ResponseBuilder, RouteContext};

use crate::api_worker::{
    AppState, api::schema::requests::AskQuestionRequest, application::AppError,
};

pub async fn ask_question_handler(
    mut req: Request,
    ctx: RouteContext<AppState>,
) -> worker::Result<Response> {
    let page = match ctx.param("page") {
        Some(p) => p.to_string(),
        None => {
            return Ok(Response::from(AppError::ValidationError(
                "No page provided".into(),
            )));
        }
    };

    if !ctx.data.config.allowed_blog_paths.contains(&page) {
        return Ok(Response::from(AppError::NotFound(format!(
            "Unknown post: {page}"
        ))));
    }

    let payload: AskQuestionRequest = match req.json().await {
        Ok(p) => p,
        Err(e) => {
            error!(error = ?e, "Failed to parse request body");
            return Ok(Response::from(AppError::ValidationError(
                "Failed to parse request body".into(),
            )));
        }
    };

    let event_stream = match ctx
        .data
        .blog_qa_service
        .answer_stream(&page, &payload.question)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            error!(error = ?e, "Ask a question setup failed");
            return Ok(Response::from(e));
        }
    };

    info!(slug = %page, "Ask a question stream opened");
    let byte_stream = event_stream.map(|event| Ok::<Vec<u8>, worker::Error>(event.encode()));

    ResponseBuilder::new()
        .with_header("Content-Type", "text/event-stream")?
        .with_header("Cache-Control", "no-cache, no-transform")?
        .with_header("X-Accel-Buffering", "no")?
        .from_stream(byte_stream)
}
