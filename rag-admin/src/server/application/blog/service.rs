use std::sync::Arc;

use tokio::sync::RwLock;

use crate::server::application::blog::ports::BlogSource;
use crate::server::application::blog::ports::PostChunkingConfigStore;
use crate::server::application::chunking::ChunkOutput;
use crate::server::application::chunking::PostChunkingService;
use crate::server::application::ingest::ports::ManifestStore;
use crate::server::application::ports::Tokenizer;
use crate::server::application::AppError;
use crate::server::domain::{Chunk, Post};
use crate::shared::{
    ChunkPreview, ChunkStrategy, ChunkingConfig, GlossaryTermDto, PostDetailDto, PostSummary,
    SettingsDto,
};

pub struct PostService {
    blog_source: Arc<dyn BlogSource>,
    manifest_store: Arc<dyn ManifestStore>,
    post_chunking_config_store: Arc<dyn PostChunkingConfigStore>,
    tokenizer: Arc<dyn Tokenizer>,
    embedding_token_limit: u32,
    settings: Arc<RwLock<SettingsDto>>,
    post_chunking_service: Arc<PostChunkingService>,
}

impl PostService {
    pub fn new(
        blog_source: Arc<dyn BlogSource>,
        manifest_store: Arc<dyn ManifestStore>,
        post_chunking_config_store: Arc<dyn PostChunkingConfigStore>,
        tokenizer: Arc<dyn Tokenizer>,
        embedding_token_limit: u32,
        settings: Arc<RwLock<SettingsDto>>,
        post_chunking_service: Arc<PostChunkingService>,
    ) -> Arc<Self> {
        Arc::new(Self {
            blog_source,
            manifest_store,
            post_chunking_config_store,
            tokenizer,
            embedding_token_limit,
            settings,
            post_chunking_service,
        })
    }

    pub async fn list_posts(&self) -> Result<Vec<PostSummary>, AppError> {
        let posts = self.blog_source.list().await?;
        let manifest = self.manifest_store.load().await?;
        let post_chunking_configs = self.post_chunking_config_store.all().await?;
        let settings = self.settings.read().await.clone();
        Ok(posts
            .into_iter()
            .map(|p| {
                let entry = manifest.posts.get(&p.slug);
                let manifest_pv = entry.map(|e| e.post_version.clone());
                let effective_chunking = post_chunking_configs
                    .get(&p.slug)
                    .copied()
                    .unwrap_or(settings.default_chunking);
                let is_dirty = entry
                    .map(|e| {
                        p.post_version != e.post_version
                            || e.chunking_config != Some(effective_chunking)
                            || e.embedding_model.as_ref() != Some(&settings.embedding_model)
                    })
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

        let settings = self.settings.read().await.clone();
        let default_chunking = settings.default_chunking;
        let post_chunking_config = self.post_chunking_config_store.get(slug).await?;
        let effective = chunking_override
            .or(post_chunking_config)
            .unwrap_or(default_chunking);
        let is_dirty = entry
            .map(|e| {
                post.is_dirty(Some(e))
                    || e.chunking_config != Some(effective)
                    || e.embedding_model.as_ref() != Some(&settings.embedding_model)
            })
            .unwrap_or(true);

        let skip_saved_llm_preview =
            chunking_override.is_none() && effective.strategy == ChunkStrategy::Llm;
        let (chunked_post, chunk_preview_notice) = if skip_saved_llm_preview {
            (
                crate::server::application::chunking::ChunkedPost {
                    body_chunks: Vec::new(),
                    glossary_chunks: post.glossary_chunks(0),
                },
                Some(
                    "LLM chunk preview skipped on initial load. Apply an explicit preview override to generate it."
                        .to_string(),
                ),
            )
        } else {
            (
                self.post_chunking_service
                    .chunk_post(&post, effective, true)
                    .await?,
                None,
            )
        };

        let mut all_previews: Vec<ChunkPreview> = chunked_post
            .body_chunks
            .iter()
            .map(|c| self.body_chunk_preview(c))
            .collect::<Result<Vec<_>, _>>()?;
        all_previews.extend(
            chunked_post
                .glossary_chunks
                .iter()
                .map(|c| self.glossary_chunk_preview(c))
                .collect::<Result<Vec<_>, _>>()?,
        );

        Ok(PostDetailDto {
            is_dirty,
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
            post_chunking_config,
            chunk_preview_notice,
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
    use crate::server::application::chunking::chunkers::BertChunker;
    use crate::server::application::chunking::chunkers::SectionChunker;
    use crate::server::application::chunking::ChunkingEngine;
    use crate::server::application::test_support::{
        make_blog_post, make_glossary_term, MockBlogSource, MockManifestStore,
        MockPostChunkingConfigStore, MockTokenizer,
    };
    use crate::server::domain::{ManifestEntry, PostVersion};
    use crate::shared::{ChunkStrategy, ChunkingConfig, SettingsDto};

    const TOKEN_LIMIT: u32 = 512;

    fn default_chunking() -> ChunkingConfig {
        ChunkingConfig {
            strategy: ChunkStrategy::Section,
            max_section_tokens: 480,
            target_tokens: 384,
            overlap_tokens: 64,
            min_tokens: 96,
            llm_micro_chunk_tokens: 96,
        }
    }

    fn markdown_parser() -> Arc<dyn crate::server::application::ports::MarkdownParser> {
        Arc::new(crate::server::infrastructure::markdown::MarkdownRsParser)
    }

    fn post_chunking_service() -> Arc<PostChunkingService> {
        let mut chunking_engine = ChunkingEngine::new(Arc::new(MockTokenizer::new()));
        chunking_engine.add(Arc::new(SectionChunker::new(markdown_parser())));
        chunking_engine.add(Arc::new(BertChunker::new(markdown_parser())));
        PostChunkingService::new(Arc::new(chunking_engine))
    }

    fn settings() -> Arc<RwLock<SettingsDto>> {
        Arc::new(RwLock::new(SettingsDto {
            default_chunking: default_chunking(),
            ..SettingsDto::default()
        }))
    }

    fn current_version_for(post: &crate::server::domain::BlogPost) -> PostVersion {
        PostVersion::compute(&post.source_markdown, &post.glossary_terms)
    }

    fn manifest_entry(post_version: String, chunk_count: u32) -> ManifestEntry {
        ManifestEntry {
            post_version,
            chunk_count,
            ingested_at: "2026-01-01T00:00:00Z".into(),
            chunking_config: Some(default_chunking()),
            embedding_model: Some(SettingsDto::default().embedding_model),
        }
    }

    #[tokio::test]
    async fn list_posts_marks_unseen_post_as_dirty() {
        let post = make_blog_post("a", "Alpha", "## Heading\n\nbody");
        let blog = Arc::new(MockBlogSource::new().with_post(post));
        let manifest = Arc::new(MockManifestStore::new());
        let svc = PostService::new(
            blog,
            manifest,
            Arc::new(MockPostChunkingConfigStore::new()),
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
            post_chunking_service(),
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
        let manifest = Arc::new(
            MockManifestStore::new().with_entry("a", manifest_entry(version.into_string(), 1)),
        );
        let svc = PostService::new(
            blog,
            manifest,
            Arc::new(MockPostChunkingConfigStore::new()),
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
            post_chunking_service(),
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
        let manifest = Arc::new(
            MockManifestStore::new().with_entry("a", manifest_entry("stale-version".into(), 1)),
        );
        let svc = PostService::new(
            blog,
            manifest,
            Arc::new(MockPostChunkingConfigStore::new()),
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
            post_chunking_service(),
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
            Arc::new(MockPostChunkingConfigStore::new()),
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
            post_chunking_service(),
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
            Arc::new(MockPostChunkingConfigStore::new()),
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
            post_chunking_service(),
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
        let body = "## A\n\n".to_string() + &"x ".repeat(5000);
        let post = make_blog_post("a", "Alpha", &body);
        let blog = Arc::new(MockBlogSource::new().with_post(post));
        let manifest = Arc::new(MockManifestStore::new());
        let svc = PostService::new(
            blog,
            manifest,
            Arc::new(MockPostChunkingConfigStore::new()),
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
            post_chunking_service(),
        );

        let override_cfg = ChunkingConfig {
            strategy: ChunkStrategy::Section,
            max_section_tokens: 100,
            ..ChunkingConfig::default()
        };
        let detail = svc.get_post_detail("a", Some(override_cfg)).await.unwrap();
        assert_eq!(detail.effective_chunking, override_cfg);
        assert!(detail.chunk_preview.len() > 1);
    }

    #[tokio::test]
    async fn get_post_detail_skips_saved_llm_preview_on_initial_load() {
        let post = make_blog_post("a", "Alpha", "## A\n\nbody");
        let llm_config = ChunkingConfig {
            strategy: ChunkStrategy::Llm,
            llm_micro_chunk_tokens: 96,
            ..ChunkingConfig::default()
        };
        let svc = PostService::new(
            Arc::new(MockBlogSource::new().with_post(post)),
            Arc::new(MockManifestStore::new()),
            Arc::new(MockPostChunkingConfigStore::new().with_config("a", llm_config)),
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
            post_chunking_service(),
        );

        let detail = svc.get_post_detail("a", None).await.unwrap();

        assert_eq!(detail.effective_chunking, llm_config);
        assert_eq!(detail.post_chunking_config, Some(llm_config));
        assert!(detail.chunk_preview_notice.is_some());
        assert!(detail.chunk_preview.is_empty());
    }

    #[tokio::test]
    async fn get_post_detail_clean_when_versions_match() {
        let post = make_blog_post("a", "Alpha", "## A\n\nbody");
        let version = current_version_for(&post);
        let blog = Arc::new(MockBlogSource::new().with_post(post));
        let manifest = Arc::new(
            MockManifestStore::new().with_entry("a", manifest_entry(version.into_string(), 1)),
        );
        let svc = PostService::new(
            blog,
            manifest,
            Arc::new(MockPostChunkingConfigStore::new()),
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
            post_chunking_service(),
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
            Arc::new(MockPostChunkingConfigStore::new()),
            Arc::new(MockTokenizer::new()),
            TOKEN_LIMIT,
            settings(),
            post_chunking_service(),
        );

        let err = svc.get_post_detail("absent", None).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
