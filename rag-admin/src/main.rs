#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use rag_admin::app::{shell, App};
    use rag_admin::server::api::sse::ingest_logs_handler;
    use rag_admin::server::setup::AppState;
    use std::sync::Arc;
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let app_state = Arc::new(
        AppState::initialize()
            .await
            .expect("failed to initialize app state"),
    );

    let app = Router::new()
        .route(
            "/api/ingest/logs/{job_id}",
            axum::routing::get(ingest_logs_handler),
        )
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            {
                let app_state = app_state.clone();
                move || provide_context(app_state.clone())
            },
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options)
        .layer(axum::Extension(app_state));

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("rag-admin listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}
