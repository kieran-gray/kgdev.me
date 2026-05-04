use std::sync::Arc;

use tokio::sync::RwLock;

use crate::server::application::chunker::{self, ChunkOutput};
use crate::server::application::ports::{BlogSource, ManifestStore, Tokenizer};
use crate::server::application::AppError;
use crate::server::domain::{Chunk, Post};
use crate::shared::{
    ChunkPreview, ChunkingConfig, GlossaryTermDto, PostDetailDto, PostSummary, SettingsDto,
};

pub struct PostService {
    blog_source: Arc<dyn BlogSource>,
    manifest_store: Arc<dyn ManifestStore>,
    tokenizer: Arc<dyn Tokenizer>,
    embedding_token_limit: u32,
    settings: Arc<RwLock<SettingsDto>>,
}

impl PostService {
    pub fn new(
        blog_source: Arc<dyn BlogSource>,
        manifest_store: Arc<dyn ManifestStore>,
        tokenizer: Arc<dyn Tokenizer>,
        embedding_token_limit: u32,
        settings: Arc<RwLock<SettingsDto>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            blog_source,
            manifest_store,
            tokenizer,
            embedding_token_limit,
            settings,
        })
    }

    pub async fn list_posts(&self) -> Result<Vec<PostSummary>, AppError> {
        let posts = self.blog_source.list().await?;
        let manifest = self.manifest_store.load().await?;
        Ok(posts
            .into_iter()
            .map(|p| {
                let entry = manifest.posts.get(&p.slug);
                let manifest_pv = entry.map(|e| e.post_version.clone());
                let is_dirty = manifest_pv
                    .as_ref()
                    .map(|m| &p.post_version != m)
                    .unwrap_or(true);
                PostSummary {
                    is_dirty,
                    manifest_post_version: manifest_pv,
                    manifest_chunk_count: entry.map(|e| e.chunk_count),
                    manifest_ingested_at: entry.map(|e| e.ingested_at.clone()),
                    slug: p.slug,
                    title: p.title,
                    published_at: p.published_at,
                }
            })
            .collect())
    }

    pub async fn get_post_detail(
        &self,
        slug: &str,
        chunking_override: Option<ChunkingConfig>,
    ) -> Result<PostDetailDto, AppError> {
        let blog_post = self.blog_source.fetch(slug).await?;
        let post = Post::try_new(blog_post)?;
        let manifest = self.manifest_store.load().await?;
        let entry = manifest.posts.get(slug);

        let default_chunking = self.settings.read().await.default_chunking;
        let effective = chunking_override.unwrap_or(default_chunking);

        let body_chunks = chunker::chunk(effective, post.markdown_body());
        let post_chunk_count = body_chunks.len() as u32;
        let glossary_chunks = post.glossary_chunks(post_chunk_count);

        let mut all_previews: Vec<ChunkPreview> = body_chunks
            .iter()
            .map(|c| self.body_chunk_preview(c))
            .collect::<Result<Vec<_>, _>>()?;
        all_previews.extend(
            glossary_chunks
                .iter()
                .map(|c| self.glossary_chunk_preview(c))
                .collect::<Result<Vec<_>, _>>()?,
        );

        Ok(PostDetailDto {
            is_dirty: post.is_dirty(entry),
            slug: post.slug().to_string(),
            title: post.title().to_string(),
            published_at: post.published_at().to_string(),
            current_post_version: post.version().as_str().to_string(),
            manifest_post_version: entry.map(|e| e.post_version.clone()),
            manifest_chunk_count: entry.map(|e| e.chunk_count),
            manifest_ingested_at: entry.map(|e| e.ingested_at.clone()),
            markdown_body_length: post.markdown_body().chars().count() as u32,
            embedding_token_limit: self.embedding_token_limit,
            effective_chunking: effective,
            default_chunking,
            glossary_terms: post
                .glossary_terms()
                .iter()
                .cloned()
                .map(|g| GlossaryTermDto {
                    slug: g.slug,
                    term: g.term,
                    definition: g.definition,
                })
                .collect(),
            chunk_preview: all_previews,
        })
    }

    fn body_chunk_preview(&self, c: &ChunkOutput) -> Result<ChunkPreview, AppError> {
        let tokenized = self.tokenizer.encode(&c.text)?;
        Ok(ChunkPreview {
            chunk_id: c.chunk_id,
            heading: heading_or_placeholder(&c.heading),
            text_excerpt: c.text.clone(),
            tokens: tokenized.tokens,
            token_count: tokenized.count,
            text_length: c.text.chars().count() as u32,
            is_glossary: false,
        })
    }

    fn glossary_chunk_preview(&self, c: &Chunk) -> Result<ChunkPreview, AppError> {
        let tokenized = self.tokenizer.encode(&c.text)?;
        Ok(ChunkPreview {
            chunk_id: c.chunk_id,
            heading: heading_or_placeholder(&c.heading),
            text_excerpt: c.text.clone(),
            tokens: tokenized.tokens,
            token_count: tokenized.count,
            text_length: c.text.chars().count() as u32,
            is_glossary: c.is_glossary,
        })
    }
}

fn heading_or_placeholder(heading: &str) -> String {
    if heading.is_empty() {
        "(no heading)".to_string()
    } else {
        heading.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::server::application::test_support::{
        make_blog_post, make_glossary_term, MockBlogSource, MockManifestStore, MockTokenizer,
    };
    use crate::server::domain::{ManifestEntry, PostVersion};
    use crate::shared::{ChunkStrategy, ChunkingConfig, SettingsDto};

    const TOKEN_LIMIT: u32 = 512;

    fn settings() -> Arc<RwLock<SettingsDto>> {
        Arc::new(RwLock::new(SettingsDto {
            default_chunking: ChunkingConfig {
                strategy: ChunkStrategy::Section,
                max_section_chars: 8000,
                target_chars: 1600,
                overlap_chars: 240,
                min_chars: 320,
            },
            ..SettingsDto::default()
        }))
    }

    fn current_version_for(post: &crate::server::domain::BlogPost) -> PostVersion {
        PostVersion::compute(&post.source_markdown, &post.glossary_terms)
    }

    #[tokio::test]
    async fn list_posts_marks_unseen_post_as_dirty() {
        let post = make_blog_post("a", "Alpha", "## Heading\n\nbody");
        let blog = Arc::new(MockBlogSource::new().with_post(post));
        let manifest = Arc::new(MockManifestStore::new());
        let svc = PostService::new(
            blog,
            manifest,
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
        );

        let summaries = svc.list_posts().await.unwrap();

        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].is_dirty);
        assert!(summaries[0].manifest_post_version.is_none());
    }

    #[tokio::test]
    async fn list_posts_marks_unchanged_post_as_clean() {
        let post = make_blog_post("a", "Alpha", "## Heading\n\nbody");
        let version = current_version_for(&post);
        let blog = Arc::new(MockBlogSource::new().with_post(post));
        let manifest = Arc::new(MockManifestStore::new().with_entry(
            "a",
            ManifestEntry {
                post_version: version.into_string(),
                chunk_count: 1,
                ingested_at: "2026-01-01T00:00:00Z".into(),
            },
        ));
        let svc = PostService::new(
            blog,
            manifest,
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
        );

        let summaries = svc.list_posts().await.unwrap();
        assert_eq!(summaries.len(), 1);
        assert!(!summaries[0].is_dirty);
        assert_eq!(summaries[0].manifest_chunk_count, Some(1));
    }

    #[tokio::test]
    async fn list_posts_marks_changed_post_as_dirty() {
        let post = make_blog_post("a", "Alpha", "## Heading\n\nbody");
        let blog = Arc::new(MockBlogSource::new().with_post(post));
        let manifest = Arc::new(MockManifestStore::new().with_entry(
            "a",
            ManifestEntry {
                post_version: "stale-version".into(),
                chunk_count: 1,
                ingested_at: "2026-01-01T00:00:00Z".into(),
            },
        ));
        let svc = PostService::new(
            blog,
            manifest,
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
        );

        let summaries = svc.list_posts().await.unwrap();
        assert!(summaries[0].is_dirty);
    }

    #[tokio::test]
    async fn get_post_detail_returns_chunks_with_token_counts() {
        let post = make_blog_post("a", "Alpha", "## Section A\n\nfirst body paragraph.");
        let blog = Arc::new(MockBlogSource::new().with_post(post));
        let manifest = Arc::new(MockManifestStore::new());
        let svc = PostService::new(
            blog,
            manifest,
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
        );

        let detail = svc.get_post_detail("a", None).await.unwrap();

        assert_eq!(detail.slug, "a");
        assert!(detail.is_dirty);
        assert!(!detail.chunk_preview.is_empty());
        assert!(detail.chunk_preview.iter().all(|c| c.token_count > 0));
        assert_eq!(detail.embedding_token_limit, TOKEN_LIMIT);
    }

    #[tokio::test]
    async fn get_post_detail_includes_glossary_chunks() {
        let mut post = make_blog_post("a", "Alpha", "## Section A\n\nbody");
        post.glossary_terms = vec![make_glossary_term("rag", "RAG")];
        let blog = Arc::new(MockBlogSource::new().with_post(post));
        let manifest = Arc::new(MockManifestStore::new());
        let svc = PostService::new(
            blog,
            manifest,
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
        );

        let detail = svc.get_post_detail("a", None).await.unwrap();

        let glossary_previews: Vec<_> = detail
            .chunk_preview
            .iter()
            .filter(|c| c.is_glossary)
            .collect();
        assert_eq!(glossary_previews.len(), 1);
        assert!(glossary_previews[0].heading.starts_with("Glossary:"));
        assert_eq!(detail.glossary_terms.len(), 1);
    }

    #[tokio::test]
    async fn get_post_detail_uses_chunking_override() {
        let body = "## A\n\n".to_string() + &"x".repeat(5000);
        let post = make_blog_post("a", "Alpha", &body);
        let blog = Arc::new(MockBlogSource::new().with_post(post));
        let manifest = Arc::new(MockManifestStore::new());
        let svc = PostService::new(
            blog,
            manifest,
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
        );

        let override_cfg = ChunkingConfig {
            strategy: ChunkStrategy::Section,
            max_section_chars: 1000,
            ..ChunkingConfig::default()
        };
        let detail = svc.get_post_detail("a", Some(override_cfg)).await.unwrap();
        assert_eq!(detail.effective_chunking, override_cfg);
        assert!(detail.chunk_preview.len() > 1);
    }

    #[tokio::test]
    async fn get_post_detail_clean_when_versions_match() {
        let post = make_blog_post("a", "Alpha", "## A\n\nbody");
        let version = current_version_for(&post);
        let blog = Arc::new(MockBlogSource::new().with_post(post));
        let manifest = Arc::new(MockManifestStore::new().with_entry(
            "a",
            ManifestEntry {
                post_version: version.into_string(),
                chunk_count: 1,
                ingested_at: "2026-01-01T00:00:00Z".into(),
            },
        ));
        let svc = PostService::new(
            blog,
            manifest,
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
        );

        let detail = svc.get_post_detail("a", None).await.unwrap();
        assert!(!detail.is_dirty);
    }

    #[tokio::test]
    async fn get_post_detail_propagates_fetch_failure() {
        let blog = Arc::new(
            MockBlogSource::new().with_fetch_failure(AppError::NotFound("missing".into())),
        );
        let svc = PostService::new(
            blog,
            Arc::new(MockManifestStore::new()),
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
        );

        let err = svc.get_post_detail("absent", None).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
