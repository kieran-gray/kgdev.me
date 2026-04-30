use async_trait::async_trait;

use crate::api_worker::{application::AppError, domain::entity::ContactMessage};

#[async_trait(?Send)]
pub trait EmailServiceTrait {
    async fn forward_contact_message(&self, message: &ContactMessage) -> Result<(), AppError>;
}
