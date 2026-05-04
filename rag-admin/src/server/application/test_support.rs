use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::Value;

use crate::server::application::ports::{
    BlogSource, Embedder, KvStore, ManifestStore, Tokenized, Tokenizer, VectorStore,
};
use crate::server::application::AppError;
use crate::server::domain::{
    BlogPost, BlogPostSummary, GlossarySource, GlossaryTerm, Manifest, ManifestEntry, PostVersion,
    VectorRecord,
};

pub fn make_blog_post(slug: &str, title: &str, body: &str) -> BlogPost {
    BlogPost {
        slug: slug.into(),
        title: title.into(),
        published_at: "2026-01-01T00:00:00Z".into(),
        source_markdown: format!("---\ntitle: {title}\n---\n{body}"),
        markdown_body: body.into(),
        plain_text: body.into(),
        glossary_terms: Vec::new(),
    }
}

pub fn make_glossary_term(slug: &str, term: &str) -> GlossaryTerm {
    GlossaryTerm {
        slug: slug.into(),
        term: term.into(),
        definition: format!("definition of {term}"),
        sources: vec![GlossarySource {
            title: "Wikipedia".into(),
            url: format!("https://en.wikipedia.org/wiki/{term}"),
        }],
    }
}

pub struct MockBlogSource {
    posts: Mutex<HashMap<String, BlogPost>>,
    fetch_failure: Mutex<Option<AppError>>,
}

impl MockBlogSource {
    pub fn new() -> Self {
        Self {
            posts: Mutex::new(HashMap::new()),
            fetch_failure: Mutex::new(None),
        }
    }

    pub fn with_post(self, post: BlogPost) -> Self {
        self.posts.lock().unwrap().insert(post.slug.clone(), post);
        self
    }

    pub fn with_fetch_failure(self, err: AppError) -> Self {
        *self.fetch_failure.lock().unwrap() = Some(err);
        self
    }
}

#[async_trait]
impl BlogSource for MockBlogSource {
    async fn list(&self) -> Result<Vec<BlogPostSummary>, AppError> {
        let posts = self.posts.lock().unwrap();
        let mut summaries: Vec<BlogPostSummary> = posts
            .values()
            .map(|p| BlogPostSummary {
                slug: p.slug.clone(),
                title: p.title.clone(),
                published_at: p.published_at.clone(),
                post_version: PostVersion::compute(&p.source_markdown, &p.glossary_terms),
            })
            .collect();
        summaries.sort_by(|a, b| a.slug.cmp(&b.slug));
        Ok(summaries)
    }

    async fn fetch(&self, slug: &str) -> Result<BlogPost, AppError> {
        if let Some(err) = self.fetch_failure.lock().unwrap().take() {
            return Err(err);
        }
        self.posts
            .lock()
            .unwrap()
            .get(slug)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("post {slug}")))
    }
}

pub struct MockEmbedder {
    dims: u32,
    actual_dims: Mutex<Option<u32>>,
    failure: Mutex<Option<AppError>>,
    calls: Mutex<Vec<(String, Vec<String>)>>,
}

impl MockEmbedder {
    pub fn new(dims: u32) -> Self {
        Self {
            dims,
            actual_dims: Mutex::new(None),
            failure: Mutex::new(None),
            calls: Mutex::new(Vec::new()),
        }
    }

    pub fn with_actual_dims(self, dims: u32) -> Self {
        *self.actual_dims.lock().unwrap() = Some(dims);
        self
    }

    pub fn with_failure(self, err: AppError) -> Self {
        *self.failure.lock().unwrap() = Some(err);
        self
    }

    pub fn calls(&self) -> Vec<(String, Vec<String>)> {
        self.calls.lock().unwrap().clone()
    }

    pub fn total_texts_embedded(&self) -> usize {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|(_, t)| t.len())
            .sum()
    }
}

#[async_trait]
impl Embedder for MockEmbedder {
    async fn embed_batch(&self, model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, AppError> {
        if let Some(err) = self.failure.lock().unwrap().take() {
            return Err(err);
        }
        self.calls
            .lock()
            .unwrap()
            .push((model.to_string(), texts.to_vec()));
        let dims = self.actual_dims.lock().unwrap().unwrap_or(self.dims) as usize;
        Ok(texts
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let mut v = vec![0.0_f32; dims];
                if dims > 0 {
                    v[0] = (i + 1) as f32;
                }
                v
            })
            .collect())
    }
}

pub struct MockVectorStore {
    upserts: Mutex<Vec<(String, Vec<VectorRecord>)>>,
    deletes: Mutex<Vec<(String, Vec<String>)>>,
}

impl MockVectorStore {
    pub fn new() -> Self {
        Self {
            upserts: Mutex::new(Vec::new()),
            deletes: Mutex::new(Vec::new()),
        }
    }

    pub fn upserts(&self) -> Vec<(String, Vec<VectorRecord>)> {
        self.upserts.lock().unwrap().clone()
    }

    pub fn deletes(&self) -> Vec<(String, Vec<String>)> {
        self.deletes.lock().unwrap().clone()
    }

    pub fn total_upserted(&self) -> usize {
        self.upserts
            .lock()
            .unwrap()
            .iter()
            .map(|(_, r)| r.len())
            .sum()
    }
}

#[async_trait]
impl VectorStore for MockVectorStore {
    async fn upsert(&self, index: &str, records: &[VectorRecord]) -> Result<(), AppError> {
        self.upserts
            .lock()
            .unwrap()
            .push((index.to_string(), records.to_vec()));
        Ok(())
    }

    async fn delete_ids(&self, index: &str, ids: &[String]) -> Result<(), AppError> {
        self.deletes
            .lock()
            .unwrap()
            .push((index.to_string(), ids.to_vec()));
        Ok(())
    }
}

pub struct MockKvStore {
    puts: Mutex<HashMap<String, Value>>,
}

impl MockKvStore {
    pub fn new() -> Self {
        Self {
            puts: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        self.puts.lock().unwrap().get(key).cloned()
    }

    pub fn len(&self) -> usize {
        self.puts.lock().unwrap().len()
    }
}

#[async_trait]
impl KvStore for MockKvStore {
    async fn put_json(&self, key: &str, value: &Value) -> Result<(), AppError> {
        self.puts
            .lock()
            .unwrap()
            .insert(key.to_string(), value.clone());
        Ok(())
    }
}

pub struct MockManifestStore {
    manifest: Mutex<Manifest>,
    records: Mutex<Vec<(String, ManifestEntry)>>,
}

impl MockManifestStore {
    pub fn new() -> Self {
        Self {
            manifest: Mutex::new(Manifest::default()),
            records: Mutex::new(Vec::new()),
        }
    }

    pub fn with_entry(self, slug: &str, entry: ManifestEntry) -> Self {
        self.manifest
            .lock()
            .unwrap()
            .posts
            .insert(slug.to_string(), entry);
        self
    }

    pub fn records(&self) -> Vec<(String, ManifestEntry)> {
        self.records.lock().unwrap().clone()
    }
}

#[async_trait]
impl ManifestStore for MockManifestStore {
    async fn load(&self) -> Result<Manifest, AppError> {
        Ok(self.manifest.lock().unwrap().clone())
    }

    async fn record(&self, slug: &str, entry: ManifestEntry) -> Result<(), AppError> {
        self.manifest
            .lock()
            .unwrap()
            .posts
            .insert(slug.to_string(), entry.clone());
        self.records.lock().unwrap().push((slug.to_string(), entry));
        Ok(())
    }
}

pub struct MockTokenizer;

impl MockTokenizer {
    pub fn new() -> Self {
        Self
    }
}

impl Tokenizer for MockTokenizer {
    fn encode(&self, text: &str) -> Result<Tokenized, AppError> {
        let tokens: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
        let count = tokens.len() as u32;
        Ok(Tokenized { tokens, count })
    }
}
