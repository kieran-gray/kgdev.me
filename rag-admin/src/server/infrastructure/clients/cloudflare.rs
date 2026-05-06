use std::sync::Arc;

use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Method;
use tokio::sync::RwLock;

use crate::server::application::AppError;
use crate::server::infrastructure::http_client::ReqwestHttpClient;
use crate::shared::SettingsDto;

pub const CLOUDFLARE_API_BASE: &str = "https://api.cloudflare.com/client/v4";

#[derive(Debug, Clone)]
pub struct CloudflareCredentials {
    pub account_id: String,
    pub api_token: String,
    pub kv_namespace_id: String,
}

impl CloudflareCredentials {
    pub fn from_settings(s: &SettingsDto) -> Result<Self, AppError> {
        if s.cloudflare_account_id.is_empty()
            || s.cloudflare_api_token.is_empty()
            || s.kv_namespace_id.is_empty()
        {
            return Err(AppError::Validation(
                "Cloudflare settings incomplete; check Settings page".into(),
            ));
        }
        Ok(Self {
            account_id: s.cloudflare_account_id.clone(),
            api_token: s.cloudflare_api_token.clone(),
            kv_namespace_id: s.kv_namespace_id.clone(),
        })
    }
}

pub struct CloudflareApi {
    http: Arc<ReqwestHttpClient>,
    settings: Arc<RwLock<SettingsDto>>,
}

impl CloudflareApi {
    pub fn new(http: Arc<ReqwestHttpClient>, settings: Arc<RwLock<SettingsDto>>) -> Self {
        Self { http, settings }
    }

    pub async fn credentials(&self) -> Result<CloudflareCredentials, AppError> {
        let s = self.settings.read().await;
        CloudflareCredentials::from_settings(&s)
    }

    fn auth_headers(token: &str, content_type: &str) -> Result<HeaderMap, AppError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|e| AppError::Internal(format!("auth header: {e}")))?,
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_str(content_type)
                .map_err(|e| AppError::Internal(format!("content-type header: {e}")))?,
        );
        Ok(headers)
    }

    pub async fn request(
        &self,
        method: Method,
        url: &str,
        body: Vec<u8>,
        content_type: &str,
        label: &str,
    ) -> Result<String, AppError> {
        let creds = self.credentials().await?;
        let headers = Self::auth_headers(&creds.api_token, content_type)?;
        let body_opt = if body.is_empty() { None } else { Some(body) };
        let (status, body_text) = self
            .http
            .request_text(method, url, headers, body_opt)
            .await?;
        if !(200..300).contains(&status) {
            return Err(AppError::Upstream(format!(
                "{label}: {status} — {}",
                truncate(&body_text, 500)
            )));
        }
        Ok(body_text)
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n).collect::<String>() + "…"
    }
}
