use std::sync::Arc;

use worker::Env;

use crate::api_worker::{
    application::{ContactMessageService, ContactMessageServiceTrait},
    infrastructure::{
        CloudflareEmailService, CloudflareRequestValidationService, HttpClientTrait,
        WorkerHttpClient, durable_object_client::DurableObjectClient,
    },
    setup::{Config, exceptions::SetupError},
};

pub struct AppState {
    pub config: Config,
    pub contact_message_service: Arc<dyn ContactMessageServiceTrait>,
    pub view_counter_do_client: DurableObjectClient,
}

impl AppState {
    pub fn from_env(env: &Env, config: Config) -> Result<Self, SetupError> {
        let http_client: Arc<dyn HttpClientTrait> = Arc::new(WorkerHttpClient::new());

        let request_validation_service = CloudflareRequestValidationService::create(
            &config.security.siteverify_url,
            &config.security.turnstile_secret_key,
            http_client.clone(),
        );

        let email_service = CloudflareEmailService::create(
            config.cloudflare.account_id.clone(),
            config.cloudflare.api_token.clone(),
            config.destination_email.clone(),
            http_client.clone(),
        );

        let contact_message_service =
            ContactMessageService::create(request_validation_service, email_service);

        let view_counter_do_client = DurableObjectClient::new(
            env.durable_object("VIEW_COUNTER")
                .map_err(|_e| SetupError::MissingVariable("VIEW_COUNTER".to_string()))?,
        );

        Ok(Self {
            config,
            contact_message_service,
            view_counter_do_client,
        })
    }
}
