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

impl From<EvaluationQuestion> for EvaluationQuestion {
    fn from(q: EvaluationQuestion) -> Self {
        Self {
            question: q.question,
            references: q.references.into_iter().map(|r| r.into()).collect(),
            embedding: q.embedding.map(crate::shared::ordered_f32_vec),
        }
    }
}

impl From<EvaluationReference> for EvaluationReference {
    fn from(r: EvaluationReference) -> Self {
        Self {
            content: r.content,
            char_start: r.char_start,
            char_end: r.char_end,
            embedding: r.embedding.map(crate::shared::ordered_f32_vec),
        }
    }
}