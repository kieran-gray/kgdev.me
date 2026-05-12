use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSweepTemplate {
    pub name: String,
    pub members: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSweepTemplate {
    pub sweep_template_id: Uuid,
    pub name: String,
    pub members: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteSweepTemplate {
    pub sweep_template_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetDefaultSweepTemplate {
    pub sweep_template_id: Uuid,
}
