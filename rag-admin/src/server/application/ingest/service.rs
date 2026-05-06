use std::collections::HashSet;
use std::sync::Arc;

use serde_json::json;
use tokio::sync::{Mutex, RwLock};

use crate::server::application::blog::ports::{BlogSource, PostChunkingConfigStore};
use crate::server::application::chunking::PostChunkingService;
use crate::server::application::ingest::ports::{KvStore, ManifestStore, VectorStore};
use crate::server::application::{embedding::EmbeddingService, AppError};
use crate::server::application::{IngestLogEvent, Job, JobRegistry};
use crate::server::domain::{ManifestEntry, Post, VectorRecord};
use crate::shared::{ChunkingConfig, IngestJobInfo, IngestOptions, SettingsDto};

const EMBED_BATCH: usize = 50;
const UPSERT_BATCH: usize = 100;

pub struct IngestService {
    blog_source: Arc<dyn BlogSource>,
    embedding_service: Arc<EmbeddingService>,
    vector_store: Arc<dyn VectorStore>,
    kv_store: Arc<dyn KvStore>,
    manifest_store: Arc<dyn ManifestStore>,
    post_chunking_config_store: Arc<dyn PostChunkingConfigStore>,
    settings: Arc<RwLock<SettingsDto>>,
    job_registry: Arc<JobRegistry>,
    post_chunking_service: Arc<PostChunkingService>,
    running: Mutex<HashSet<String>>,
}

pub struct IngestServiceDeps {
    pub blog_source: Arc<dyn BlogSource>,
    pub embedding_service: Arc<EmbeddingService>,
    pub vector_store: Arc<dyn VectorStore>,
    pub kv_store: Arc<dyn KvStore>,
    pub manifest_store: Arc<dyn ManifestStore>,
    pub post_chunking_config_store: Arc<dyn PostChunkingConfigStore>,
    pub settings: Arc<RwLock<SettingsDto>>,
    pub job_registry: Arc<JobRegistry>,
    pub post_chunking_service: Arc<PostChunkingService>,
}

impl IngestService {
    pub fn new(deps: IngestServiceDeps) -> Arc<Self> {
        Arc::new(Self {
            blog_source: deps.blog_source,
            embedding_service: deps.embedding_service,
            vector_store: deps.vector_store,
            kv_store: deps.kv_store,
            manifest_store: deps.manifest_store,
            post_chunking_config_store: deps.post_chunking_config_store,
            settings: deps.settings,
            job_registry: deps.job_registry,
            post_chunking_service: deps.post_chunking_service,
            running: Mutex::new(HashSet::new()),
        })
    }

    pub async fn start_ingest(
        self: &Arc<Self>,
        slug: String,
        options: IngestOptions,
    ) -> Result<IngestJobInfo, AppError> {
        {
            let mut guard = self.running.lock().await;
            if guard.contains(&slug) {
                return Err(AppError::Validation(format!(
                    "ingestion for {slug} is already running"
                )));
            }
            guard.insert(slug.clone());
        }

        let (job_id, job) = self.job_registry.create().await;
        let stream_url = format!("/api/ingest/logs/{job_id}");

        let svc = self.clone();
        let slug_for_task = slug.clone();
        let job_for_task = job.clone();
        tokio::spawn(async move {
            let result = svc
                .run_ingest(&slug_for_task, options, job_for_task.clone())
                .await;
            if let Err(e) = result {
                job_for_task
                    .emit(IngestLogEvent::error(format!("ingest failed: {e}")))
                    .await;
            }
            job_for_task.finish().await;
            svc.running.lock().await.remove(&slug_for_task);
        });

        Ok(IngestJobInfo { job_id, stream_url })
    }

    async fn run_ingest(
        &self,
        slug: &str,
        options: IngestOptions,
        job: Arc<Job>,
    ) -> Result<(), AppError> {
        job.emit(IngestLogEvent::info(format!(
            "fetching post {slug} from blog…"
        )))
        .await;
        let blog_post = self.blog_source.fetch(slug).await?;
        let post = Post::try_new(blog_post)?;

        let manifest = self.manifest_store.load().await?;
        let prev = manifest.posts.get(slug).cloned();

        let (vector_index, model, default_chunking) = {
            let s = self.settings.read().await;
            (
                s.vector_index.clone(),
                s.embedding_model.clone(),
                s.default_chunking,
            )
        };
        let post_chunking_config = self.post_chunking_config_store.get(slug).await?;
        let effective = options
            .chunking_override
            .or(post_chunking_config)
            .unwrap_or(default_chunking);

        if let Some(options) = options.chunking_override {
            job.emit(IngestLogEvent::info(format!(
                "using one-shot chunking override: {}",
                describe_chunking(options)
            )))
            .await;
        }
        let chunked_post = self
            .post_chunking_service
            .chunk_post(&post, effective, true)
            .await?;
        let post_chunk_count = chunked_post.body_chunk_count();
        let glossary_chunk_count = chunked_post.glossary_chunk_count();
        let chunk_count = chunked_post.total_chunk_count();
        let chunks = chunked_post.into_chunks();

        let unchanged_content = matches!(
            (&prev, options.force),
            (Some(p), false) if post.version() == &p.post_version
        );
        let chunking_unchanged = options.chunking_override.is_none()
            && prev
                .as_ref()
                .and_then(|p| p.chunking_config)
                .map(|c| c == effective)
                .unwrap_or(false);
        let embedding_model_unchanged = prev
            .as_ref()
            .and_then(|p| p.embedding_model.as_ref())
            .map(|m| m == &model)
            .unwrap_or(false);
        let was_seen = unchanged_content && chunking_unchanged && embedding_model_unchanged;

        if was_seen {
            job.emit(IngestLogEvent::info(format!(
                "{slug}: unchanged ({}), skipping",
                post.version().short()
            )))
            .await;
            if !options.dry_run {
                self.kv_store
                    .put_json(
                        &format!("post_version:{slug}"),
                        &json!({ "v": post.version().as_str() }),
                    )
                    .await?;
                job.emit(IngestLogEvent::info("KV post_version refreshed"))
                    .await;
            }
            job.emit(IngestLogEvent::success("done (skipped)")).await;
            return Ok(());
        }

        let prev_count_str = prev
            .as_ref()
            .map(|p| format!(" (was {})", p.chunk_count))
            .unwrap_or_default();
        job.emit(IngestLogEvent::info(format!(
            "{slug}: {} chunks + {} glossary{} @ {}",
            post_chunk_count,
            glossary_chunk_count,
            prev_count_str,
            post.version().short()
        )))
        .await;

        if options.dry_run {
            job.emit(IngestLogEvent::success(format!(
                "dry run complete · {chunk_count} total chunks would be upserted"
            )))
            .await;
            return Ok(());
        }

        let mut embeddings: Vec<Vec<f32>> = Vec::with_capacity(chunks.len());
        let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        for (i, batch) in texts.chunks(EMBED_BATCH).enumerate() {
            job.emit(IngestLogEvent::info(format!(
                "embedding batch {}/{} ({} chunks) via {} ({})…",
                i + 1,
                texts.len().div_ceil(EMBED_BATCH),
                batch.len(),
                model.backend.as_str(),
                model.id
            )))
            .await;
            let vecs = self.embedding_service.embed_batch(&model, batch).await?;
            embeddings.extend(vecs);
        }

        let records: Vec<VectorRecord> = chunks
            .iter()
            .zip(embeddings)
            .map(|(c, values)| VectorRecord {
                id: post.vector_id(c),
                values,
                metadata: post.metadata_for(c),
            })
            .collect();

        for (i, batch) in records.chunks(UPSERT_BATCH).enumerate() {
            job.emit(IngestLogEvent::info(format!(
                "upserting batch {}/{} ({} records) → index '{}'…",
                i + 1,
                records.len().div_ceil(UPSERT_BATCH),
                batch.len(),
                vector_index.name()
            )))
            .await;
            self.vector_store.upsert(vector_index.name(), batch).await?;
        }

        if let Some(p) = &prev {
            if p.chunk_count > chunk_count {
                let stale: Vec<String> = (chunk_count..p.chunk_count)
                    .map(|i| format!("{}:{}", slug, i))
                    .collect();
                job.emit(IngestLogEvent::info(format!(
                    "deleting {} stale vector(s)",
                    stale.len()
                )))
                .await;
                self.vector_store
                    .delete_ids(vector_index.name(), &stale)
                    .await?;
            }
        }

        self.kv_store
            .put_json(
                &format!("post_version:{slug}"),
                &json!({ "v": post.version().as_str() }),
            )
            .await?;

        let ingested_at = now_rfc3339();
        self.manifest_store
            .record(
                slug,
                ManifestEntry {
                    post_version: post.version().as_str().to_string(),
                    chunk_count,
                    ingested_at: ingested_at.clone(),
                    chunking_config: Some(effective),
                    embedding_model: Some(model),
                },
            )
            .await?;

        job.emit(IngestLogEvent::success(format!(
            "ingest complete · {chunk_count} chunks · version {}",
            post.version().short()
        )))
        .await;

        Ok(())
    }
}

fn describe_chunking(c: ChunkingConfig) -> String {
    c.describe()
}

fn now_rfc3339() -> String {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::server::application::chunking::chunkers::SectionChunker;
    use crate::server::application::chunking::ChunkingEngine;
    use crate::server::application::test_support::{
        make_blog_post, make_glossary_term, MockBlogSource, MockEmbedder, MockKvStore,
        MockManifestStore, MockPostChunkingConfigStore, MockTokenizer, MockVectorStore,
    };
    use crate::server::domain::PostVersion;
    use crate::shared::{
        ChunkStrategy, ChunkingConfig, EmbedderBackend, EmbeddingModel, VectorIndexConfig,
    };

    const VECTOR_INDEX: &str = "test-index";
    const TEST_DIMS: u32 = 4;

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

    fn embedding_model() -> EmbeddingModel {
        EmbeddingModel {
            backend: EmbedderBackend::Cloudflare,
            id: "@cf/test/model".into(),
            dims: TEST_DIMS,
        }
    }

    fn manifest_entry(post_version: String, chunk_count: u32) -> ManifestEntry {
        ManifestEntry {
            post_version,
            chunk_count,
            ingested_at: "2026-01-01T00:00:00Z".into(),
            chunking_config: Some(default_chunking()),
            embedding_model: Some(embedding_model()),
        }
    }

    fn markdown_parser() -> Arc<dyn crate::server::application::ports::MarkdownParser> {
        Arc::new(crate::server::infrastructure::markdown::MarkdownRsParser)
    }

    struct Fixture {
        service: Arc<IngestService>,
        embedder: Arc<MockEmbedder>,
        vectors: Arc<MockVectorStore>,
        kv: Arc<MockKvStore>,
        manifest: Arc<MockManifestStore>,
        job_registry: Arc<JobRegistry>,
    }

    impl Fixture {
        fn build(blog: MockBlogSource, manifest: MockManifestStore) -> Self {
            let blog: Arc<dyn BlogSource> = Arc::new(blog);
            let embedder = Arc::new(MockEmbedder::new(TEST_DIMS));
            let vectors = Arc::new(MockVectorStore::new());
            let kv = Arc::new(MockKvStore::new());
            let manifest = Arc::new(manifest);
            let post_chunking_config_store = Arc::new(MockPostChunkingConfigStore::new());
            let job_registry = Arc::new(JobRegistry::new());

            let settings = Arc::new(RwLock::new(SettingsDto {
                vector_index: VectorIndexConfig::Cloudflare {
                    name: VECTOR_INDEX.into(),
                    dimensions: TEST_DIMS,
                },
                embedding_model: embedding_model(),
                default_chunking: default_chunking(),
                ..SettingsDto::default()
            }));

            let mut chunking_engine = ChunkingEngine::new(Arc::new(MockTokenizer::new()));
            chunking_engine.add(Arc::new(SectionChunker::new(markdown_parser())));
            let post_chunking_service = PostChunkingService::new(Arc::new(chunking_engine));

            let embedding_service = EmbeddingService::new(embedder.clone());

            let service = IngestService::new(IngestServiceDeps {
                blog_source: blog,
                embedding_service,
                vector_store: vectors.clone(),
                kv_store: kv.clone(),
                manifest_store: manifest.clone(),
                post_chunking_config_store,
                settings,
                job_registry: job_registry.clone(),
                post_chunking_service,
            });

            Self {
                service,
                embedder,
                vectors,
                kv,
                manifest,
                job_registry,
            }
        }

        async fn job(&self) -> Arc<Job> {
            self.job_registry.create().await.1
        }
    }

    #[tokio::test]
    async fn new_post_embeds_upserts_and_records_manifest() {
        let post = make_blog_post("a", "Alpha", "## A\n\nfirst.\n\n## B\n\nsecond.");
        let fx = Fixture::build(
            MockBlogSource::new().with_post(post.clone()),
            MockManifestStore::new(),
        );
        let job = fx.job().await;

        fx.service
            .run_ingest("a", IngestOptions::default(), job)
            .await
            .unwrap();

        let upserts = fx.vectors.upserts();
        assert_eq!(upserts.len(), 1);
        assert_eq!(upserts[0].0, VECTOR_INDEX);
        let upserted = &upserts[0].1;
        assert!(!upserted.is_empty());
        assert!(upserted.iter().all(|r| r.id.starts_with("a:")));

        assert!(fx.kv.get("post_version:a").is_some());

        let recorded = fx.manifest.records();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].0, "a");
        assert_eq!(recorded[0].1.chunk_count as usize, upserted.len());
        let expected_version =
            PostVersion::compute(&post.source_markdown, &post.glossary_terms).into_string();
        assert_eq!(recorded[0].1.post_version, expected_version);
    }

    #[tokio::test]
    async fn unchanged_post_skips_embed_and_upsert_but_refreshes_kv() {
        let post = make_blog_post("a", "Alpha", "## A\n\nbody");
        let version = PostVersion::compute(&post.source_markdown, &post.glossary_terms);
        let fx = Fixture::build(
            MockBlogSource::new().with_post(post),
            MockManifestStore::new().with_entry("a", manifest_entry(version.into_string(), 1)),
        );

        fx.service
            .run_ingest("a", IngestOptions::default(), fx.job().await)
            .await
            .unwrap();

        assert_eq!(fx.embedder.calls().len(), 0);
        assert!(fx.vectors.upserts().is_empty());
        assert_eq!(fx.kv.len(), 1);
        assert!(fx.manifest.records().is_empty());
    }

    #[tokio::test]
    async fn force_re_ingests_even_when_unchanged() {
        let post = make_blog_post("a", "Alpha", "## A\n\nbody");
        let version = PostVersion::compute(&post.source_markdown, &post.glossary_terms);
        let fx = Fixture::build(
            MockBlogSource::new().with_post(post),
            MockManifestStore::new().with_entry("a", manifest_entry(version.into_string(), 1)),
        );

        fx.service
            .run_ingest(
                "a",
                IngestOptions {
                    force: true,
                    ..IngestOptions::default()
                },
                fx.job().await,
            )
            .await
            .unwrap();

        assert!(!fx.embedder.calls().is_empty());
        assert!(!fx.vectors.upserts().is_empty());
        assert_eq!(fx.manifest.records().len(), 1);
    }

    #[tokio::test]
    async fn dry_run_does_not_write_anything() {
        let post = make_blog_post("a", "Alpha", "## A\n\nbody");
        let fx = Fixture::build(
            MockBlogSource::new().with_post(post),
            MockManifestStore::new(),
        );

        fx.service
            .run_ingest(
                "a",
                IngestOptions {
                    dry_run: true,
                    ..IngestOptions::default()
                },
                fx.job().await,
            )
            .await
            .unwrap();

        assert_eq!(fx.embedder.calls().len(), 0);
        assert!(fx.vectors.upserts().is_empty());
        assert_eq!(fx.kv.len(), 0);
        assert!(fx.manifest.records().is_empty());
    }

    #[tokio::test]
    async fn deletes_stale_vector_ids_when_chunk_count_shrinks() {
        let post = make_blog_post("a", "Alpha", "## Only\n\nbody");
        let fx = Fixture::build(
            MockBlogSource::new().with_post(post),
            MockManifestStore::new().with_entry("a", manifest_entry("stale".into(), 5)),
        );

        fx.service
            .run_ingest("a", IngestOptions::default(), fx.job().await)
            .await
            .unwrap();

        let upserted_count = fx.vectors.upserts()[0].1.len() as u32;
        let deletes = fx.vectors.deletes();
        assert_eq!(deletes.len(), 1);
        assert_eq!(deletes[0].0, VECTOR_INDEX);
        let stale_ids: Vec<String> = (upserted_count..5).map(|i| format!("a:{i}")).collect();
        assert_eq!(deletes[0].1, stale_ids);
    }

    #[tokio::test]
    async fn no_stale_delete_when_chunk_count_grows() {
        let body = "## A\n\nfirst.\n\n## B\n\nsecond.\n\n## C\n\nthird.";
        let post = make_blog_post("a", "Alpha", body);
        let fx = Fixture::build(
            MockBlogSource::new().with_post(post),
            MockManifestStore::new().with_entry("a", manifest_entry("stale".into(), 1)),
        );

        fx.service
            .run_ingest("a", IngestOptions::default(), fx.job().await)
            .await
            .unwrap();

        assert!(fx.vectors.deletes().is_empty());
    }

    #[tokio::test]
    async fn chunking_override_forces_re_embed_even_when_version_matches() {
        let post = make_blog_post("a", "Alpha", "## A\n\nbody");
        let version = PostVersion::compute(&post.source_markdown, &post.glossary_terms);
        let fx = Fixture::build(
            MockBlogSource::new().with_post(post),
            MockManifestStore::new().with_entry("a", manifest_entry(version.into_string(), 1)),
        );

        let override_cfg = ChunkingConfig {
            strategy: ChunkStrategy::Section,
            max_section_tokens: 200,
            ..ChunkingConfig::default()
        };

        fx.service
            .run_ingest(
                "a",
                IngestOptions {
                    chunking_override: Some(override_cfg),
                    ..IngestOptions::default()
                },
                fx.job().await,
            )
            .await
            .unwrap();

        assert!(!fx.embedder.calls().is_empty());
        assert!(!fx.vectors.upserts().is_empty());
    }

    #[tokio::test]
    async fn embeds_glossary_chunks_alongside_body() {
        let mut post = make_blog_post("a", "Alpha", "## A\n\nbody");
        post.glossary_terms = vec![
            make_glossary_term("rag", "RAG"),
            make_glossary_term("kv", "KV"),
        ];
        let fx = Fixture::build(
            MockBlogSource::new().with_post(post),
            MockManifestStore::new(),
        );

        fx.service
            .run_ingest("a", IngestOptions::default(), fx.job().await)
            .await
            .unwrap();

        let upserted = &fx.vectors.upserts()[0].1;
        let glossary_records: Vec<_> = upserted
            .iter()
            .filter(|r| {
                r.metadata
                    .get("heading")
                    .and_then(|v| v.as_str())
                    .map(|s| s.starts_with("Glossary:"))
                    .unwrap_or(false)
            })
            .collect();
        assert_eq!(glossary_records.len(), 2);
        assert_eq!(fx.embedder.total_texts_embedded(), upserted.len());
    }

    #[tokio::test]
    async fn rejects_post_with_empty_title() {
        let mut post = make_blog_post("a", "", "## A\n\nbody");
        post.title = "".into();
        let fx = Fixture::build(
            MockBlogSource::new().with_post(post),
            MockManifestStore::new(),
        );

        let err = fx
            .service
            .run_ingest("a", IngestOptions::default(), fx.job().await)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[tokio::test]
    async fn propagates_blog_source_failure() {
        let fx = Fixture::build(
            MockBlogSource::new().with_fetch_failure(AppError::Upstream("offline".into())),
            MockManifestStore::new(),
        );

        let err = fx
            .service
            .run_ingest("missing", IngestOptions::default(), fx.job().await)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Upstream(_)));
        assert!(fx.embedder.calls().is_empty());
    }

    #[tokio::test]
    async fn start_ingest_rejects_concurrent_run_for_same_slug() {
        let post = make_blog_post("a", "Alpha", "## A\n\nbody");
        let fx = Fixture::build(
            MockBlogSource::new().with_post(post),
            MockManifestStore::new(),
        );

        // Reserve the slug as already running.
        fx.service.running.lock().await.insert("a".into());

        let err = fx
            .service
            .start_ingest("a".into(), IngestOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[tokio::test]
    async fn start_ingest_returns_job_info_for_free_slug() {
        let post = make_blog_post("a", "Alpha", "## A\n\nbody");
        let fx = Fixture::build(
            MockBlogSource::new().with_post(post),
            MockManifestStore::new(),
        );

        let info = fx
            .service
            .start_ingest("a".into(), IngestOptions::default())
            .await
            .unwrap();
        assert!(!info.job_id.is_empty());
        assert!(info.stream_url.contains(&info.job_id));
    }
}
