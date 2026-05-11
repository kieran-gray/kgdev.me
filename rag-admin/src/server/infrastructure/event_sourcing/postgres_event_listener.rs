use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use sqlx::postgres::PgListener;
use sqlx::PgPool;
use tokio::sync::Notify;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Spawns a background task that listens for `events_appended` notifications
/// and pings the matching `Notify` per `aggregate_type`.
///
/// One listener per process; all projection drivers register their wake-up
/// `Notify` against the aggregate types they care about. On reconnect the
/// listener re-establishes; the projection drivers' 2s heartbeat catches any
/// notifications missed during the gap.
pub fn spawn_postgres_event_listener(pool: PgPool, wakeups: HashMap<String, Arc<Notify>>) {
    tokio::spawn(async move {
        loop {
            match PgListener::connect_with(&pool).await {
                Ok(mut listener) => {
                    if let Err(e) = listener.listen("events_appended").await {
                        error!(error = %e, "postgres listener: LISTEN failed; retrying");
                        sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                    info!("postgres listener: connected, awaiting events_appended");
                    loop {
                        match listener.recv().await {
                            Ok(notification) => {
                                let aggregate_type = notification.payload();
                                debug!(aggregate_type, "postgres listener: events_appended notify");
                                if let Some(notify) = wakeups.get(aggregate_type) {
                                    notify.notify_one();
                                } else {
                                    debug!(
                                        aggregate_type,
                                        "postgres listener: no driver registered for aggregate type"
                                    );
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "postgres listener: recv error; reconnecting");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "postgres listener: connect failed; retrying in 2s");
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    });
}
