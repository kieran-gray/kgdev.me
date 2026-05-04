use std::sync::Arc;

use async_trait::async_trait;
use reqwest::header::HeaderMap;
use reqwest::Method;
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::server::application::ports::BlogSource;
use crate::server::application::AppError;
use crate::server::domain::{BlogPost, BlogPostSummary, GlossarySource, GlossaryTerm, PostVersion};
use crate::server::infrastructure::http_client::ReqwestHttpClient;
use crate::shared::SettingsDto;

pub struct HttpBlogSource {
    http: Arc<ReqwestHttpClient>,
    settings: Arc<RwLock<SettingsDto>>,
}

impl HttpBlogSource {
    pub fn new(http: Arc<ReqwestHttpClient>, settings: Arc<RwLock<SettingsDto>>) -> Arc<Self> {
        Arc::new(Self { http, settings })
    }

    async fn base_url(&self) -> Result<String, AppError> {
        let s = self.settings.read().await;
        let url = s.blog_url.trim().trim_end_matches('/').to_string();
        if url.is_empty() {
            return Err(AppError::Validation(
                "blog URL is not configured (Settings → Blog base URL)".into(),
            ));
        }
        Ok(url)
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, AppError> {
        let (status, body) = self
            .http
            .request_text(Method::GET, url, HeaderMap::new(), None)
            .await?;
        if !(200..300).contains(&status) {
            return Err(AppError::Upstream(format!(
                "GET {url} returned {status}: {}",
                truncate(&body, 300)
            )));
        }
        serde_json::from_str(&body).map_err(|e| AppError::Upstream(format!("parse {url}: {e}")))
    }
}

#[derive(Debug, Deserialize)]
struct PostsListResponse {
    posts: Vec<PostSummaryWire>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PostSummaryWire {
    slug: String,
    title: String,
    published_at: String,
    content_hash: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PostDetailWire {
    slug: String,
    title: String,
    published_at: String,
    source_markdown: String,
    markdown_body: String,
    plain_text: String,
    glossary_terms: Vec<GlossaryTermWire>,
}

#[derive(Debug, Deserialize)]
struct GlossaryTermWire {
    slug: String,
    term: String,
    definition: String,
    sources: Vec<GlossarySourceWire>,
}

#[derive(Debug, Deserialize)]
struct GlossarySourceWire {
    title: String,
    url: String,
}

#[async_trait]
impl BlogSource for HttpBlogSource {
    async fn list(&self) -> Result<Vec<BlogPostSummary>, AppError> {
        let base = self.base_url().await?;
        let url = format!("{base}/api/posts/index.json");
        let res: PostsListResponse = self.get_json(&url).await?;
        Ok(res
            .posts
            .into_iter()
            .map(|p| BlogPostSummary {
                slug: p.slug,
                title: p.title,
                published_at: p.published_at,
                post_version: PostVersion::from_hex(p.content_hash),
            })
            .collect())
    }

    async fn fetch(&self, slug: &str) -> Result<BlogPost, AppError> {
        let base = self.base_url().await?;
        let url = format!("{base}/api/posts/{slug}.json");
        let detail: PostDetailWire = self.get_json(&url).await?;
        let glossary_terms = detail
            .glossary_terms
            .into_iter()
            .map(|g| GlossaryTerm {
                slug: g.slug,
                term: g.term,
                definition: g.definition,
                sources: g
                    .sources
                    .into_iter()
                    .map(|s| GlossarySource {
                        title: s.title,
                        url: s.url,
                    })
                    .collect(),
            })
            .collect();
        Ok(BlogPost {
            slug: detail.slug,
            title: detail.title,
            published_at: detail.published_at,
            source_markdown: detail.source_markdown,
            markdown_body: detail.markdown_body,
            plain_text: detail.plain_text,
            glossary_terms,
        })
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n).collect::<String>() + "…"
    }
}
