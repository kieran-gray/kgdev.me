//! Legacy settings DTO. After the configuration aggregate rewrite this holds
//! only **evaluation defaults** — embedding model, vector index, and default
//! chunking now live in the `Configuration` aggregate registry and are looked
//! up by id at runtime.
//!
//! The shape is preserved (rather than renamed) to keep the
//! `/api/load_settings` and `/api/save_settings` server functions stable for
//! the existing legacy settings page; the UI overhaul will replace this
//! surface with a dedicated evaluation-defaults page.

use serde::{Deserialize, Serialize};

use super::evaluation::EvaluationSettings;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SettingsDto {
    #[serde(default)]
    pub evaluation: EvaluationSettings,
}
