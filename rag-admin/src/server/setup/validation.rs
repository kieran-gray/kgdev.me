//! Cross-aggregate validation helpers that don't fit inside a single
//! aggregate's `handle_command` path.
//!
//! Per-provider model-id format checks now live inside the `Configuration`
//! aggregate (see `validate_model_id_format`) because the provider's kind is
//! the source of truth for whether a model identifier is well-formed.

// Intentionally empty: model-id well-formedness moved into the aggregate.
