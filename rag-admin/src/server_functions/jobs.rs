use leptos::prelude::*;

use crate::shared::ActivityJobDto;

/// Snapshot of in-flight and recently-completed jobs feeding the activity
/// drawer. Server-side projection lives in
/// `server::application::ActivityRegistry`; on a fresh page load this is what
/// populates the drawer before any websocket events arrive.
#[server(name = ListActiveJobs, prefix = "/api", endpoint = "list_active_jobs")]
pub async fn list_active_jobs() -> Result<Vec<ActivityJobDto>, ServerFnError> {
    use crate::server::setup::AppState;
    use std::sync::Arc;

    let state: Arc<AppState> =
        use_context::<Arc<AppState>>().ok_or_else(|| ServerFnError::new("missing app state"))?;
    Ok(state.activity_registry.snapshot().await)
}
