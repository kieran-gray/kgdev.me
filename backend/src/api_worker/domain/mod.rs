pub mod contact_message;
pub mod question;

pub use contact_message::{entity::ContactMessage, exceptions::ContactMessageValidationError};
pub use question::{entity::Question, exceptions::QuestionValidationError};
