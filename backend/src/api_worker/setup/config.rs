use std::str::FromStr;
use worker::Env;

use crate::api_worker::setup::exceptions::SetupError;

#[derive(Clone)]
pub struct Config {
    pub siteverify_url: String,
    pub turnstile_secret_key: String,
    pub allowed_origins: Vec<String>,
    pub allowed_blog_paths: Vec<String>,
    pub cloudflare_account_id: String,
    pub cloudflare_api_token: String,
    pub destination_email: String,
}

impl Config {
    pub fn from_env(env: &Env) -> Result<Self, SetupError> {
        let siteverify_url = Config::parse(env, "CLOUDFLARE_SITEVERIFY_URL")?;
        let turnstile_secret_key = Config::parse(env, "CLOUDFLARE_TURNSTILE_SECRET_KEY")?;
        let allowed_origins = Config::parse_csv(env, "ALLOWED_ORIGINS")?;
        let allowed_blog_paths = Config::parse_csv(env, "ALLOWED_BLOG_PATHS")?;
        let cloudflare_account_id = Config::parse(env, "CLOUDFLARE_ACCOUNT_ID")?;
        let cloudflare_api_token = Config::parse(env, "CLOUDFLARE_EMAIL_API_TOKEN")?;
        let destination_email = Config::parse(env, "DESTINATION_EMAIL")?;

        Ok(Config {
            siteverify_url,
            turnstile_secret_key,
            allowed_origins,
            allowed_blog_paths,
            cloudflare_account_id,
            cloudflare_api_token,
            destination_email,
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
