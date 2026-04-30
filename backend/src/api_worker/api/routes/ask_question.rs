use serde::Serialize;
use tracing::{error, info};
use worker::{Request, Response, RouteContext};

use crate::api_worker::{
    AppState,
    api::{cors::CorsContext, schema::requests::AskQuestionRequest},
    application::{AnswerResult, AppError, CachedSource},
};

#[derive(Serialize)]
struct AskQuestionResponse {
    answer: String,
    sources: Vec<CachedSource>,
    model: String,
    cached: bool,
}

impl From<AnswerResult> for AskQuestionResponse {
    fn from(value: AnswerResult) -> Self {
        Self {
            answer: value.answer,
            sources: value.sources,
            model: value.model,
            cached: value.cached,
        }
    }
}

pub async fn ask_question_handler(
    mut req: Request,
    ctx: RouteContext<AppState>,
    cors_context: CorsContext,
) -> worker::Result<Response> {
    let page = match ctx.param("page") {
        Some(p) => p.to_string(),
        None => {
            let response = Response::from(AppError::ValidationError("No page provided".into()));
            return Ok(cors_context.add_to_response(response));
        }
    };

    if !ctx.data.config.allowed_blog_paths.contains(&page) {
        let response = Response::from(AppError::NotFound(format!("Unknown post: {page}")));
        return Ok(cors_context.add_to_response(response));
    }

    let payload: AskQuestionRequest = match req.json().await {
        Ok(p) => p,
        Err(e) => {
            error!(error = ?e, "Failed to parse request body");
            let response = Response::from(AppError::ValidationError(
                "Failed to parse request body".into(),
            ));
            return Ok(cors_context.add_to_response(response));
        }
    };

    match ctx
        .data
        .blog_qa_service
        .answer(&page, &payload.question)
        .await
    {
        Ok(result) => {
            info!(slug = %page, cached = result.cached, "qa answered");
            let response: AskQuestionResponse = result.into();
            let response = Response::from_json(&response)?;
            Ok(cors_context.add_to_response(response))
        }
        Err(e) => {
            error!(error = ?e, "qa failed");
            let response = Response::from(e);
            Ok(cors_context.add_to_response(response))
        }
    }
}
