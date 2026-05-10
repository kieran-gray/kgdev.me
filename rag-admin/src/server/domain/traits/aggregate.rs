// Re-export the canonical Aggregate trait from the event_sourcing module so
// existing `use crate::server::domain::Aggregate` imports keep working while
// the trait itself lives next to its companions (EventStore, AggregateRepository,
// CommandProcessor) in `server::event_sourcing`.
pub use crate::server::event_sourcing::Aggregate;
