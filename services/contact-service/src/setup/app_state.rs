use std::sync::Arc;

use worker::Env;

use crate::{
    application::{ContactMessageService, ContactMessageServiceTrait},
    infrastructure::{
        CloudflareEmailService, CloudflareRequestValidationService, WorkerHttpClient,
    },
    setup::{Config, exceptions::SetupError},
};

pub struct AppState {
    pub config: Config,
    pub contact_message_service: Arc<dyn ContactMessageServiceTrait>,
}

impl AppState {
    pub fn from_env(_env: &Env, config: Config) -> Result<Self, SetupError> {
        let http_client = Arc::new(WorkerHttpClient::new());

        let request_validation_service = CloudflareRequestValidationService::create(
            &config.siteverify_url,
            &config.turnstile_secret_key,
            http_client.clone(),
        );

        let email_service = CloudflareEmailService::create(
            config.cloudflare_account_id.clone(),
            config.cloudflare_api_token.clone(),
            config.destination_email.clone(),
            http_client.clone(),
        );

        let contact_message_service =
            ContactMessageService::create(request_validation_service, email_service);

        Ok(Self {
            config,
            contact_message_service,
        })
    }
}
