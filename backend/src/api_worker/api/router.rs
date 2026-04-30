use crate::api_worker::{
    api::{
        middleware::{create_options_handler, public},
        routes::{
            ask_question::ask_question_handler, connect_websocket::handle_websocket_connect,
            create_contact_message::create_contact_message_handler,
        },
    },
    setup::AppState,
};

use worker::Router;

pub fn create_router(app_state: AppState) -> Router<'static, AppState> {
    let router = Router::with_data(app_state);
    router
        .post_async("/api/v1/contact/", |req, ctx| {
            public(create_contact_message_handler, req, ctx)
        })
        .options("/api/v1/contact/", create_options_handler)
        .post_async("/api/v1/ask/:page", |req, ctx| {
            public(ask_question_handler, req, ctx)
        })
        .options("/api/v1/ask/:page", create_options_handler)
        .on_async("/api/v1/connect/:page", |req, ctx| {
            public(handle_websocket_connect, req, ctx)
        })
}
