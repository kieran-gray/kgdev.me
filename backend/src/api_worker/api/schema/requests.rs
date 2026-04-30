use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Deserialize, Serialize)]
pub struct CreateContactMessageRequest {
    pub token: String,
    pub email: String,
    pub name: String,
    pub message: String,
}

#[derive(PartialEq, Debug, Deserialize, Serialize)]
pub struct AskQuestionRequest {
    pub question: String,
}
