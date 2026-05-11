//! Shared CRUD plumbing for the `Configuration` aggregate (providers,
//! embedding/generation models, vector indexes, and pipeline configurations).
//!
//! Views live in the page files that consume this module:
//! - `pages::pipelines` — Pipeline Configurations
//! - `pages::settings`  — the registry sections (Providers, Models, Indexes)
//!
//! The legacy tabbed `NewSettingsPage` view that bundled all five concerns
//! into one screen is intentionally removed. Each page now owns its own
//! presentation and reuses `commands::run_configuration_command` for writes.

pub mod commands;
pub mod dialogs;
