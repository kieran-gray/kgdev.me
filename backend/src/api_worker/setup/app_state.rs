use std::sync::Arc;

use worker::Env;

use crate::api_worker::{
    application::{
        BlogQaService, BlogQaServiceTrait, ContactMessageService, ContactMessageServiceTrait,
        QaCacheService,
    },
    infrastructure::{
        CloudflareEmailService, CloudflareRequestValidationService, KVCache,
        QaCoordinatorDoService, VectorizeRestService, WorkerHttpClient, WorkersAiService,
        durable_object_client::DurableObjectClient,
    },
    setup::{Config, exceptions::SetupError},
};

const QA_CACHE_TTL_SECONDS: u64 = 60 * 60 * 24 * 30;

pub struct AppState {
    pub config: Config,
    pub contact_message_service: Arc<dyn ContactMessageServiceTrait>,
    pub blog_qa_service: Arc<dyn BlogQaServiceTrait>,
    pub view_counter_do_client: DurableObjectClient,
}

impl AppState {
    pub fn from_env(env: &Env, config: Config) -> Result<Self, SetupError> {
        let http_client = Arc::new(WorkerHttpClient::new());

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

        let ai_binding = env
            .ai("AI")
            .map_err(|_| SetupError::MissingVariable("AI".to_string()))?;
        let ai_service = WorkersAiService::create(
            ai_binding,
            config.ai.embedding_model.clone(),
            config.ai.generation_model.clone(),
        );

        let vectorize_service = VectorizeRestService::create(
            config.cloudflare.account_id.clone(),
            config.cloudflare.vectorize_api_token.clone(),
            config.ai.vectorize_index_name.clone(),
            http_client.clone(),
        );

        let kv = env
            .kv("BLOG_POST_QA_CACHE")
            .map_err(|_| SetupError::MissingVariable("BLOG_POST_QA_CACHE".to_string()))?;
        let kv_cache = Arc::new(KVCache::create(kv, QA_CACHE_TTL_SECONDS));
        let qa_cache_service = QaCacheService::create(kv_cache);

        let qa_do_client = DurableObjectClient::new(
            env.durable_object("BLOG_POST_QA")
                .map_err(|_| SetupError::MissingVariable("BLOG_POST_QA".to_string()))?,
        );
        let qa_coordinator = QaCoordinatorDoService::create(qa_do_client);

        let blog_qa_service = BlogQaService::create(
            ai_service,
            vectorize_service,
            qa_cache_service,
            qa_coordinator,
            config.ai.generation_model.clone(),
            config.qa_daily_cap,
            config.ai.vectorize_top_k,
            config.ai.min_score,
        );

        let view_counter_do_client = DurableObjectClient::new(
            env.durable_object("VIEW_COUNTER")
                .map_err(|_| SetupError::MissingVariable("VIEW_COUNTER".to_string()))?,
        );

        Ok(Self {
            config,
            contact_message_service,
            blog_qa_service,
            view_counter_do_client,
        })
    }
}
