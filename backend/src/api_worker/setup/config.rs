use std::str::FromStr;
use worker::Env;

use crate::api_worker::setup::exceptions::SetupError;

trait FromEnv: Sized {
    fn from_env(env: &Env) -> Result<Self, SetupError>;
}

#[derive(Clone)]
pub struct Config {
    pub security: SecurityConfig,
    pub cloudflare: CloudflareConfig,
    pub ai: AiConfig,
    pub destination_email: String,
    pub qa_daily_cap: u32,
}

#[derive(Clone)]
pub struct SecurityConfig {
    pub siteverify_url: String,
    pub turnstile_secret_key: String,
    pub allowed_origins: Vec<String>,
    pub allowed_blog_paths: Vec<String>,
}

impl FromEnv for SecurityConfig {
    fn from_env(env: &Env) -> Result<Self, SetupError> {
        let siteverify_url = Config::parse(env, "CLOUDFLARE_SITEVERIFY_URL")?;
        let turnstile_secret_key = Config::parse(env, "CLOUDFLARE_TURNSTILE_SECRET_KEY")?;
        let allowed_origins = Config::parse_csv(env, "ALLOWED_ORIGINS")?;
        let allowed_blog_paths = Config::parse_csv(env, "ALLOWED_BLOG_PATHS")?;

        Ok(Self {
            siteverify_url,
            turnstile_secret_key,
            allowed_origins,
            allowed_blog_paths,
        })
    }
}

#[derive(Clone)]
pub struct CloudflareConfig {
    pub account_id: String,
    pub api_token: String,
    pub vectorize_api_token: String,
}

impl FromEnv for CloudflareConfig {
    fn from_env(env: &Env) -> Result<Self, SetupError> {
        let account_id = Config::parse(env, "CLOUDFLARE_ACCOUNT_ID")?;
        let api_token = Config::parse(env, "CLOUDFLARE_EMAIL_API_TOKEN")?;
        let vectorize_api_token = Config::parse(env, "CLOUDFLARE_VECTORIZE_API_TOKEN")?;

        Ok(Self {
            account_id,
            api_token,
            vectorize_api_token,
        })
    }
}

#[derive(Clone)]
pub struct AiConfig {
    pub inference: InferenceConfig,
    pub vectorize_index_name: String,
    pub vectorize_top_k: u32,
    pub min_score: f32,
}

#[derive(Clone)]
pub enum InferenceConfig {
    Cloudflare {
        embedding_model: String,
        generation_model: String,
    },

    #[cfg(feature = "ollama")]
    Ollama {
        url: String,
        embedding_model: String,
        generation_model: String,
    },
}

impl InferenceConfig {
    pub fn generation_model(&self) -> &str {
        match self {
            Self::Cloudflare {
                generation_model, ..
            } => generation_model,
            #[cfg(feature = "ollama")]
            Self::Ollama {
                generation_model, ..
            } => generation_model,
        }
    }
}

impl FromEnv for AiConfig {
    fn from_env(env: &Env) -> Result<Self, SetupError> {
        let embedding_model = Config::parse(env, "EMBEDDING_MODEL")?;
        let generation_model = Config::parse(env, "GENERATION_MODEL")?;
        let inference = InferenceConfig::from_env(env, embedding_model, generation_model)?;
        let vectorize_index_name = Config::parse(env, "VECTORIZE_INDEX_NAME")?;
        let vectorize_top_k = Config::parse(env, "VECTORIZE_TOP_K")?;
        let min_score = Config::parse(env, "MIN_SCORE")?;

        Ok(Self {
            inference,
            vectorize_index_name,
            vectorize_top_k,
            min_score,
        })
    }
}

impl InferenceConfig {
    #[cfg(not(feature = "ollama"))]
    fn from_env(
        _env: &Env,
        embedding_model: String,
        generation_model: String,
    ) -> Result<Self, SetupError> {
        Ok(Self::Cloudflare {
            embedding_model,
            generation_model,
        })
    }

    #[cfg(feature = "ollama")]
    fn from_env(
        env: &Env,
        embedding_model: String,
        generation_model: String,
    ) -> Result<Self, SetupError> {
        let provider = env
            .var("AI_PROVIDER")
            .map(|v| v.to_string())
            .unwrap_or_else(|_| "cloudflare".to_string());

        match provider.as_str() {
            "cloudflare" => Ok(Self::Cloudflare {
                embedding_model,
                generation_model,
            }),
            "ollama" => {
                let url = Config::parse(env, "OLLAMA_HOST")?;
                Ok(Self::Ollama {
                    url,
                    embedding_model,
                    generation_model,
                })
            }
            other => Err(SetupError::InvalidVariable(format!(
                "AI_PROVIDER should be cloudflare or ollama, got {other}"
            ))),
        }
    }
}

impl Config {
    pub fn from_env(env: &Env) -> Result<Self, SetupError> {
        let security = SecurityConfig::from_env(env)?;
        let cloudflare = CloudflareConfig::from_env(env)?;
        let ai = AiConfig::from_env(env)?;

        let destination_email = Config::parse(env, "DESTINATION_EMAIL")?;
        let qa_daily_cap = Config::parse(env, "QA_DAILY_CAP")?;

        Ok(Config {
            security,
            cloudflare,
            ai,
            destination_email,
            qa_daily_cap,
        })
    }

    fn parse<T: FromStr>(env: &Env, var: &str) -> Result<T, SetupError> {
        let type_name = std::any::type_name::<T>();
        let env_var: T = env
            .var(var)
            .map_err(|e| SetupError::MissingVariable(e.to_string()))?
            .to_string()
            .parse()
            .map_err(|_| SetupError::InvalidVariable(format!("{var} should be {type_name}")))?;
        Ok(env_var)
    }

    fn parse_csv(env: &Env, var: &str) -> Result<Vec<String>, SetupError> {
        let env_var = env
            .var(var)
            .map_err(|_| SetupError::MissingVariable(var.to_string()))?
            .to_string()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(env_var)
    }
}
