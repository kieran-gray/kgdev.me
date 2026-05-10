use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvaluationReference {
    pub content: String,
    pub char_start: u32,
    pub char_end: u32,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvaluationQuestion {
    pub sequence: u32,
    pub question: String,
    pub references: Vec<EvaluationReference>,
    pub embedding: Option<Vec<f32>>,
}
