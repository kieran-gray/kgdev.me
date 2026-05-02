use std::collections::HashSet;
use std::sync::Arc;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, RwLock};

use crate::server::application::chunker::{chunk, ChunkOutput};
use crate::server::application::ingest_log::IngestLogEvent;
use crate::server::application::job_registry::{Job, JobRegistry};
use crate::server::application::ports::{
    BlogSource, Embedder, KvStore, ManifestStore, Tokenizer, VectorStore,
};
use crate::server::application::AppError;
use crate::server::domain::{GlossaryTerm, ManifestEntry, VectorRecord};
use crate::shared::{
    ChunkPreview, GlossaryTermDto, IngestJobInfo, IngestOptions, PostDetailDto, PostSummary,
    SettingsDto,
};

const EMBED_BATCH: usize = 50;
const UPSERT_BATCH: usize = 100;

pub struct IngestService {
    blog_source: Arc<dyn BlogSource>,
    embedder: Arc<dyn Embedder>,
    vector_store: Arc<dyn VectorStore>,
    kv_store: Arc<dyn KvStore>,
    manifest_store: Arc<dyn ManifestStore>,
    tokenizer: Arc<dyn Tokenizer>,
    embedding_token_limit: u32,
    settings: Arc<RwLock<SettingsDto>>,
    job_registry: Arc<JobRegistry>,
    running: Mutex<HashSet<String>>,
}

impl IngestService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        blog_source: Arc<dyn BlogSource>,
        embedder: Arc<dyn Embedder>,
        vector_store: Arc<dyn VectorStore>,
        kv_store: Arc<dyn KvStore>,
        manifest_store: Arc<dyn ManifestStore>,
        tokenizer: Arc<dyn Tokenizer>,
        embedding_token_limit: u32,
        settings: Arc<RwLock<SettingsDto>>,
        job_registry: Arc<JobRegistry>,
    ) -> Arc<Self> {
        Arc::new(Self {
            blog_source,
            embedder,
            vector_store,
            kv_store,
            manifest_store,
            tokenizer,
            embedding_token_limit,
            settings,
            job_registry,
            running: Mutex::new(HashSet::new()),
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
                PostSummary {
                    is_dirty: manifest_pv
                        .as_ref()
                        .map(|m| m != &p.content_hash)
                        .unwrap_or(true),
                    manifest_post_version: manifest_pv,
                    manifest_chunk_count: entry.map(|e| e.chunk_count),
                    manifest_ingested_at: entry.map(|e| e.ingested_at.clone()),
                    slug: p.slug,
                    title: p.title,
                    content_hash: p.content_hash,
                    published_at: p.published_at,
                }
            })
            .collect())
    }

    pub async fn get_post_detail(&self, slug: &str) -> Result<PostDetailDto, AppError> {
        let post = self.blog_source.fetch(slug).await?;
        let manifest = self.manifest_store.load().await?;
        let entry = manifest.posts.get(slug);

        let post_chunks = chunk(&post.source_markdown);
        let glossary_chunks = build_glossary_chunks(&post.glossary_terms, post_chunks.len() as u32);
        let post_version = compute_post_version(&post.source_markdown, &post.glossary_terms);

        let mut all_previews: Vec<ChunkPreview> = post_chunks
            .iter()
            .map(|c| self.chunk_preview(c, false))
            .collect::<Result<Vec<_>, _>>()?;
        all_previews.extend(
            glossary_chunks
                .iter()
                .map(|c| self.chunk_preview(c, true))
                .collect::<Result<Vec<_>, _>>()?,
        );

        Ok(PostDetailDto {
            is_dirty: entry
                .map(|e| e.post_version != post_version)
                .unwrap_or(true),
            slug: post.slug.clone(),
            title: post.title.clone(),
            published_at: post.published_at.clone(),
            current_post_version: post_version,
            manifest_post_version: entry.map(|e| e.post_version.clone()),
            manifest_chunk_count: entry.map(|e| e.chunk_count),
            manifest_ingested_at: entry.map(|e| e.ingested_at.clone()),
            markdown_body_length: post.markdown_body.chars().count() as u32,
            plain_text_excerpt: excerpt(&post.plain_text, 400),
            embedding_token_limit: self.embedding_token_limit,
            glossary_terms: post
                .glossary_terms
                .iter()
                .map(|g| GlossaryTermDto {
                    slug: g.slug.clone(),
                    term: g.term.clone(),
                    definition_excerpt: excerpt(&g.definition, 280),
                })
                .collect(),
            chunk_preview: all_previews,
        })
    }

    fn chunk_preview(&self, c: &ChunkOutput, is_glossary: bool) -> Result<ChunkPreview, AppError> {
        let tokenized = self.tokenizer.encode(&c.text)?;
        Ok(ChunkPreview {
            chunk_id: c.chunk_id,
            heading: if c.heading.is_empty() {
                "(no heading)".to_string()
            } else {
                c.heading.clone()
            },
            text_excerpt: c.text.clone(),
            tokens: tokenized.tokens,
            token_count: tokenized.count,
            text_length: c.text.chars().count() as u32,
            is_glossary,
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
        let post = self.blog_source.fetch(slug).await?;

        let manifest = self.manifest_store.load().await?;
        let prev = manifest.posts.get(slug).cloned();

        let (index_name, embedding_model) = {
            let s = self.settings.read().await;
            (s.vectorize_index_name.clone(), s.embedding_model.clone())
        };

        let post_chunks = chunk(&post.source_markdown);
        let glossary_chunks = build_glossary_chunks(&post.glossary_terms, post_chunks.len() as u32);
        let chunks: Vec<ChunkOutput> = post_chunks
            .iter()
            .cloned()
            .chain(glossary_chunks.iter().cloned())
            .collect();
        let chunk_count = chunks.len() as u32;
        let post_version = compute_post_version(&post.source_markdown, &post.glossary_terms);

        let was_seen =
            matches!((&prev, options.force), (Some(p), false) if p.post_version == post_version);

        if was_seen {
            job.emit(IngestLogEvent::info(format!(
                "{slug}: unchanged ({}), skipping",
                short_hash(&post_version)
            )))
            .await;
            if !options.dry_run {
                self.kv_store
                    .put_json(
                        &format!("post_version:{slug}"),
                        &json!({ "v": post_version }),
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
            post_chunks.len(),
            glossary_chunks.len(),
            prev_count_str,
            short_hash(&post_version)
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
                "embedding batch {}/{} ({} chunks)…",
                i + 1,
                texts.len().div_ceil(EMBED_BATCH),
                batch.len()
            )))
            .await;
            let vecs = self.embedder.embed_batch(&embedding_model, batch).await?;
            embeddings.extend(vecs);
        }

        let records: Vec<VectorRecord> = chunks
            .iter()
            .zip(embeddings)
            .map(|(c, values)| {
                let metadata = build_metadata(
                    slug,
                    &post_version,
                    &post.title,
                    c,
                    glossary_sources_for(&post.glossary_terms, c, post_chunks.len() as u32),
                );
                VectorRecord {
                    id: vector_id(slug, c.chunk_id),
                    values,
                    metadata,
                }
            })
            .collect();

        for (i, batch) in records.chunks(UPSERT_BATCH).enumerate() {
            job.emit(IngestLogEvent::info(format!(
                "upserting batch {}/{} ({} records)…",
                i + 1,
                records.len().div_ceil(UPSERT_BATCH),
                batch.len()
            )))
            .await;
            self.vector_store.upsert(&index_name, batch).await?;
        }

        if let Some(p) = &prev {
            if p.chunk_count > chunk_count {
                let stale: Vec<String> = (chunk_count..p.chunk_count)
                    .map(|i| vector_id(slug, i))
                    .collect();
                job.emit(IngestLogEvent::info(format!(
                    "deleting {} stale vector(s)",
                    stale.len()
                )))
                .await;
                self.vector_store.delete_ids(&index_name, &stale).await?;
            }
        }

        self.kv_store
            .put_json(
                &format!("post_version:{slug}"),
                &json!({ "v": post_version }),
            )
            .await?;

        let ingested_at = now_rfc3339();
        self.manifest_store
            .record(
                slug,
                ManifestEntry {
                    post_version: post_version.clone(),
                    chunk_count,
                    ingested_at: ingested_at.clone(),
                },
            )
            .await?;

        job.emit(IngestLogEvent::success(format!(
            "ingest complete · {chunk_count} chunks · version {}",
            short_hash(&post_version)
        )))
        .await;

        Ok(())
    }
}

fn build_glossary_chunks(terms: &[GlossaryTerm], post_chunk_count: u32) -> Vec<ChunkOutput> {
    terms
        .iter()
        .enumerate()
        .map(|(i, t)| ChunkOutput {
            chunk_id: post_chunk_count + i as u32,
            heading: format!("Glossary: {}", t.term),
            text: format!("{}\n\n{}", t.term, t.definition),
            char_start: 0,
            char_end: 0,
        })
        .collect()
}

fn glossary_sources_for<'a>(
    terms: &'a [GlossaryTerm],
    c: &ChunkOutput,
    post_chunk_count: u32,
) -> Option<&'a [crate::server::domain::GlossarySource]> {
    if c.chunk_id < post_chunk_count {
        return None;
    }
    let idx = (c.chunk_id - post_chunk_count) as usize;
    let term = terms.get(idx)?;
    if term.sources.is_empty() {
        None
    } else {
        Some(&term.sources)
    }
}

fn build_metadata(
    slug: &str,
    post_version: &str,
    title: &str,
    c: &ChunkOutput,
    sources: Option<&[crate::server::domain::GlossarySource]>,
) -> Value {
    let mut m = json!({
        "post_slug": slug,
        "post_version": post_version,
        "post_title": title,
        "chunk_id": c.chunk_id,
        "heading": c.heading,
        "text": c.text,
        "char_start": c.char_start,
        "char_end": c.char_end,
    });
    if let Some(srcs) = sources {
        m["sources"] =
            Value::String(serde_json::to_string(srcs).unwrap_or_else(|_| "[]".to_string()));
    }
    m
}

fn vector_id(slug: &str, chunk_id: u32) -> String {
    format!("{slug}:{chunk_id}")
}

pub fn compute_post_version(source: &str, glossary: &[GlossaryTerm]) -> String {
    let glossary_json = serde_json::to_string(glossary).unwrap_or_else(|_| "[]".to_string());
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update(glossary_json.as_bytes());
    hex::encode(hasher.finalize())
}

fn short_hash(s: &str) -> String {
    if s.len() <= 8 {
        s.to_string()
    } else {
        s[..8].to_string()
    }
}

fn now_rfc3339() -> String {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn excerpt(s: &str, max_chars: usize) -> String {
    let trimmed = s.trim();
    let count = trimmed.chars().count();
    if count <= max_chars {
        return trimmed.to_string();
    }
    let mut out: String = trimmed.chars().take(max_chars).collect();
    out.push('…');
    out
}

