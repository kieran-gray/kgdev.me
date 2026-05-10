use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Extension, Query};
use axum::response::Response;
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::server::setup::AppState;

#[derive(Debug, Deserialize)]
pub struct EventsWsQuery {
    pub stream_id: Uuid,
}

/// WebSocket endpoint that fans projected domain events out to a client filtered
/// by `stream_id` (= aggregate id). Clients use the messages purely as cache
/// invalidation hints and re-query the read model on receipt.
pub async fn events_ws_handler(
    Extension(state): Extension<Arc<AppState>>,
    Query(query): Query<EventsWsQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state, query.stream_id))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>, stream_id: Uuid) {
    let mut subscription = state.event_bus.subscribe();

    loop {
        let event = match subscription.recv().await {
            Ok(event) => event,
            Err(RecvError::Lagged(n)) => {
                warn!(stream_id = %stream_id, dropped = n, "events ws subscriber lagged");
                continue;
            }
            Err(RecvError::Closed) => {
                debug!(stream_id = %stream_id, "events bus closed");
                return;
            }
        };

        if event.stream_id != stream_id {
            continue;
        }

        let payload = match serde_json::to_string(event.as_ref()) {
            Ok(p) => p,
            Err(e) => {
                warn!(error = %e, "failed to serialize PublishedEvent for ws");
                continue;
            }
        };

        if socket.send(Message::Text(payload.into())).await.is_err() {
            return;
        }
    }
}
