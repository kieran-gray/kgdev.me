use serde::{de::DeserializeOwned, Serialize};

/// Event-sourced aggregate.
///
/// State derives entirely from the ordered application of `Event`s; commands
/// produce zero-or-more events without mutating state directly. The aggregate's
/// only job is to enforce invariants — projection-only fields (lists, counts,
/// payloads that exist purely so the read model can be rebuilt) belong in
/// projectors, not here.
pub trait Aggregate: Sized + Clone + Serialize + DeserializeOwned + Send + Sync {
    type Event: Clone + Serialize + DeserializeOwned + Send + Sync + 'static;
    type Command: Send + Sync + 'static;
    type Error: std::error::Error + Send + Sync + 'static;

    /// Stable string identifier for the aggregate type. Persisted alongside
    /// each event so projectors and the event log can filter by type.
    fn aggregate_type() -> &'static str;

    /// Apply a single event to mutate state. Must be deterministic.
    fn apply(&mut self, event: &Self::Event);

    /// Decide what events (if any) a command produces. Pure — never mutates state.
    fn handle_command(
        state: Option<&Self>,
        command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error>;

    /// Reconstruct an aggregate from its full event stream. Returns `None` if
    /// the stream is empty or the first event is not a valid creation event.
    fn from_events(events: &[Self::Event]) -> Option<Self>;
}
