use std::{any::type_name, str::FromStr};

use serde::Deserialize;
use worker::Env;

use crate::api_worker::{domain::PostSlug, setup::exceptions::SetupError};

trait FromEnv: Sized {
    fn from_env(env: &Env) -> Result<Self, SetupError>;
}

const BLOG_MANIFEST_JSON: &str = include_str!("../../../generated/blog-manifest.json");

#[derive(Deserialize)]
struct BlogManifest {
    posts: Vec<String>,
}

#[derive(Clone)]
pub struct Config {
    pub security: SecurityConfig,
    pub cloudflare: CloudflareConfig,
    pub destination_email: String,
}

#[derive(Clone)]
pub struct SecurityConfig {
    pub siteverify_url: String,
    pub turnstile_secret_key: String,
    pub allowed_origins: Vec<String>,
    pub allowed_blog_paths: Vec<PostSlug>,
}

impl FromEnv for SecurityConfig {
    fn from_env(env: &Env) -> Result<Self, SetupError> {
        let siteverify_url = Config::parse(env, "CLOUDFLARE_SITEVERIFY_URL")?;
        let turnstile_secret_key = Config::parse(env, "CLOUDFLARE_TURNSTILE_SECRET_KEY")?;
        let allowed_origins = Config::parse_csv(env, "ALLOWED_ORIGINS")?;
        let allowed_blog_paths = match Config::parse_optional_slug_csv(env, "ALLOWED_BLOG_PATHS")? {
            Some(slugs) => slugs,
            None => Config::blog_manifest_slugs()?,
        };

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
}

impl FromEnv for CloudflareConfig {
    fn from_env(env: &Env) -> Result<Self, SetupError> {
        let account_id = Config::parse(env, "CLOUDFLARE_ACCOUNT_ID")?;
        let api_token = Config::parse(env, "CLOUDFLARE_EMAIL_API_TOKEN")?;

        Ok(Self {
            account_id,
            api_token,
        })
    }
}

impl Config {
    pub fn from_env(env: &Env) -> Result<Self, SetupError> {
        let security = SecurityConfig::from_env(env)?;
        let cloudflare = CloudflareConfig::from_env(env)?;

        let destination_email = Config::parse(env, "DESTINATION_EMAIL")?;

        Ok(Config {
            security,
            cloudflare,
            destination_email,
        })
    }

    fn parse<T: FromStr>(env: &Env, var: &str) -> Result<T, SetupError> {
        let type_name = type_name::<T>();
        let env_var: T = env
            .var(var)
            .map_err(|e| SetupError::MissingVariable(e.to_string()))?
            .to_string()
            .parse()
            .map_err(|_e| SetupError::InvalidVariable(format!("{var} should be {type_name}")))?;
        Ok(env_var)
    }

    fn parse_csv(env: &Env, var: &str) -> Result<Vec<String>, SetupError> {
        let env_var = env
            .var(var)
            .map_err(|_e| SetupError::MissingVariable(var.to_string()))?
            .to_string()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(env_var)
    }

    fn parse_optional_slug_csv(env: &Env, var: &str) -> Result<Option<Vec<PostSlug>>, SetupError> {
        let Some(raw) = env.var(var).ok().map(|v| v.to_string()) else {
            return Ok(None);
        };

        let slugs: Vec<String> = raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if slugs.is_empty() {
            return Ok(None);
        }

        slugs
            .into_iter()
            .map(|slug| {
                PostSlug::parse(&slug)
                    .map_err(|err| SetupError::InvalidVariable(format!("{var}: {err}")))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    fn blog_manifest_slugs() -> Result<Vec<PostSlug>, SetupError> {
        parse_blog_manifest(BLOG_MANIFEST_JSON)
    }
}

fn parse_blog_manifest(raw: &str) -> Result<Vec<PostSlug>, SetupError> {
    let manifest: BlogManifest = serde_json::from_str(raw)
        .map_err(|err| SetupError::InvalidVariable(format!("blog manifest invalid: {err}")))?;

    manifest
        .posts
        .into_iter()
        .map(|slug| {
            PostSlug::parse(&slug)
                .map_err(|err| SetupError::InvalidVariable(format!("blog manifest: {err}")))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_manifest_slugs() {
        let slugs = parse_blog_manifest(r#"{ "posts": ["first-post", "second-post"] }"#).unwrap();
        let raw: Vec<&str> = slugs.iter().map(PostSlug::as_str).collect();
        assert_eq!(raw, vec!["first-post", "second-post"]);
    }

    #[test]
    fn accepts_empty_manifest() {
        let slugs = parse_blog_manifest(r#"{ "posts": [] }"#).unwrap();
        assert!(slugs.is_empty());
    }

    #[test]
    fn rejects_invalid_manifest_slug() {
        let result = parse_blog_manifest(r#"{ "posts": ["BadSlug"] }"#);
        assert!(matches!(result, Err(SetupError::InvalidVariable(_))));
    }
}
