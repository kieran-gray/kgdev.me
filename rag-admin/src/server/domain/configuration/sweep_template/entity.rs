use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepTemplate {
    pub sweep_template_id: Uuid,
    pub name: String,
    pub members: Vec<Uuid>,
}
