use std::str::FromStr;

use super::exceptions::SetupError;

trait FromEnv: Sized {
    fn from_env() -> Result<Self, SetupError>;
}

#[derive(Clone)]
pub struct Config {
    pub blog_url: String,
    pub database_url: String,
    pub cloudflare: CloudflareConfig,
    pub ollama: OllamaConfig,
}

#[derive(Clone)]
pub struct CloudflareConfig {
    pub account_id: String,
    pub api_token: String,
    pub kv_namespace_id: String,
}

#[derive(Clone)]
pub struct OllamaConfig {
    pub base_url: String,
}

impl FromEnv for CloudflareConfig {
    fn from_env() -> Result<Self, SetupError> {
        Ok(Self {
            account_id: Config::parse("CLOUDFLARE_ACCOUNT_ID")?,
            api_token: Config::parse("CLOUDFLARE_API_TOKEN")?,
            kv_namespace_id: Config::parse("CLOUDFLARE_KV_NAMESPACE_ID")?,
        })
    }
}

impl FromEnv for OllamaConfig {
    fn from_env() -> Result<Self, SetupError> {
        Ok(Self {
            base_url: Config::parse_optional("OLLAMA_BASE_URL")
                .unwrap_or_else(|| "http://localhost:11434".into()),
        })
    }
}

impl Config {
    pub fn from_env() -> Result<Self, SetupError> {
        Ok(Self {
            blog_url: Self::parse("BLOG_URL")?,
            database_url: Self::parse("DATABASE_URL")?,
            cloudflare: CloudflareConfig::from_env()?,
            ollama: OllamaConfig::from_env()?,
        })
    }

    fn parse<T: FromStr>(var: &str) -> Result<T, SetupError> {
        let type_name = std::any::type_name::<T>();
        std::env::var(var)
            .map_err(|_| SetupError::MissingVariable(var.to_string()))?
            .parse()
            .map_err(|_| SetupError::InvalidVariable(format!("{var} must be {type_name}")))
    }

    fn parse_optional<T: FromStr>(var: &str) -> Option<T> {
        std::env::var(var).ok()?.parse().ok()
    }
}
