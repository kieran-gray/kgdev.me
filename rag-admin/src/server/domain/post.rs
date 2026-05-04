use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{BlogPost, Chunk, GlossaryTerm, ManifestEntry};
use crate::server::application::AppError;

/// SHA-256 over `source_markdown` concatenated with the JSON serialization of
/// the post's glossary terms. The frontend computes the same value and serves
/// it as `contentHash`; field order on `GlossaryTerm`/`GlossarySource` keeps
/// the two implementations in sync.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PostVersion(String);

impl PostVersion {
    pub fn compute(source_markdown: &str, glossary: &[GlossaryTerm]) -> Self {
        let glossary_json = serde_json::to_string(glossary).unwrap_or_else(|_| "[]".to_string());
        let mut hasher = Sha256::new();
        hasher.update(source_markdown.as_bytes());
        hasher.update(glossary_json.as_bytes());
        Self(hex::encode(hasher.finalize()))
    }

    pub fn from_hex(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn short(&self) -> &str {
        if self.0.len() <= 8 {
            &self.0
        } else {
            &self.0[..8]
        }
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for PostVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<str> for PostVersion {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<String> for PostVersion {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

/// A `Post` aggregate: the fetched `BlogPost` plus its derived `PostVersion`.
/// Owns the rules for dirty detection, glossary-chunk assembly, and vector
/// metadata shaping. Construct with `Post::try_new` — it validates the
/// invariant that slug and title are non-empty.
#[derive(Debug)]
pub struct Post {
    blog_post: BlogPost,
    version: PostVersion,
}

impl Post {
    pub fn try_new(blog_post: BlogPost) -> Result<Self, AppError> {
        if blog_post.slug.trim().is_empty() {
            return Err(AppError::Validation("post slug must not be empty".into()));
        }
        if blog_post.title.trim().is_empty() {
            return Err(AppError::Validation(format!(
                "post {} is missing a title",
                blog_post.slug
            )));
        }
        let version = PostVersion::compute(&blog_post.source_markdown, &blog_post.glossary_terms);
        Ok(Self { blog_post, version })
    }

    pub fn slug(&self) -> &str {
        &self.blog_post.slug
    }

    pub fn title(&self) -> &str {
        &self.blog_post.title
    }

    pub fn published_at(&self) -> &str {
        &self.blog_post.published_at
    }

    pub fn markdown_body(&self) -> &str {
        &self.blog_post.markdown_body
    }

    pub fn plain_text(&self) -> &str {
        &self.blog_post.plain_text
    }

    pub fn glossary_terms(&self) -> &[GlossaryTerm] {
        &self.blog_post.glossary_terms
    }

    pub fn version(&self) -> &PostVersion {
        &self.version
    }

    pub fn is_dirty(&self, manifest_entry: Option<&ManifestEntry>) -> bool {
        match manifest_entry {
            Some(e) => self.version != e.post_version,
            None => true,
        }
    }

    /// Build the glossary chunks that get appended after the body chunks.
    /// `start_id` is the next chunk_id after the last body chunk, so that
    /// every chunk in the post has a unique sequential id.
    pub fn glossary_chunks(&self, start_id: u32) -> Vec<Chunk> {
        self.blog_post
            .glossary_terms
            .iter()
            .enumerate()
            .map(|(i, t)| Chunk {
                chunk_id: start_id + i as u32,
                heading: format!("Glossary: {}", t.term),
                text: format!("{}\n\n{}", t.term, t.definition),
                char_start: 0,
                char_end: 0,
                sources: t.sources.clone(),
                is_glossary: true,
            })
            .collect()
    }

    pub fn vector_id(&self, chunk: &Chunk) -> String {
        format!("{}:{}", self.blog_post.slug, chunk.chunk_id)
    }

    pub fn metadata_for(&self, chunk: &Chunk) -> Value {
        let mut m = json!({
            "post_slug": self.blog_post.slug,
            "post_version": self.version.as_str(),
            "post_title": self.blog_post.title,
            "chunk_id": chunk.chunk_id,
            "heading": chunk.heading,
            "text": chunk.text,
            "char_start": chunk.char_start,
            "char_end": chunk.char_end,
        });
        if !chunk.sources.is_empty() {
            m["sources"] = Value::String(
                serde_json::to_string(&chunk.sources).unwrap_or_else(|_| "[]".to_string()),
            );
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::domain::{GlossarySource, GlossaryTerm};

    fn blog(slug: &str, title: &str, source: &str, glossary: Vec<GlossaryTerm>) -> BlogPost {
        BlogPost {
            slug: slug.into(),
            title: title.into(),
            published_at: "2026-01-01".into(),
            source_markdown: source.into(),
            markdown_body: source.into(),
            plain_text: source.into(),
            glossary_terms: glossary,
        }
    }

    fn term(slug: &str, term: &str) -> GlossaryTerm {
        GlossaryTerm {
            slug: slug.into(),
            term: term.into(),
            definition: format!("def of {term}"),
            sources: vec![GlossarySource {
                title: "Wikipedia".into(),
                url: "https://en.wikipedia.org".into(),
            }],
        }
    }

    #[test]
    fn rejects_empty_slug() {
        let err = Post::try_new(blog("", "title", "body", vec![])).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn rejects_empty_title() {
        let err = Post::try_new(blog("slug", "   ", "body", vec![])).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn version_is_stable() {
        let a = Post::try_new(blog("s", "t", "body", vec![term("g", "G")])).unwrap();
        let b = Post::try_new(blog("s", "t", "body", vec![term("g", "G")])).unwrap();
        assert_eq!(a.version(), b.version());
    }

    #[test]
    fn version_changes_on_body_change() {
        let a = Post::try_new(blog("s", "t", "one", vec![])).unwrap();
        let b = Post::try_new(blog("s", "t", "two", vec![])).unwrap();
        assert_ne!(a.version(), b.version());
    }

    #[test]
    fn version_changes_on_glossary_change() {
        let a = Post::try_new(blog("s", "t", "body", vec![term("g", "G")])).unwrap();
        let b = Post::try_new(blog("s", "t", "body", vec![])).unwrap();
        assert_ne!(a.version(), b.version());
    }

    #[test]
    fn version_ignores_glossary_slug() {
        let a = Post::try_new(blog("s", "t", "body", vec![term("g1", "Same")])).unwrap();
        let b = Post::try_new(blog("s", "t", "body", vec![term("g2", "Same")])).unwrap();
        assert_eq!(a.version(), b.version());
    }

    #[test]
    fn is_dirty_when_no_manifest_entry() {
        let p = Post::try_new(blog("s", "t", "body", vec![])).unwrap();
        assert!(p.is_dirty(None));
    }

    #[test]
    fn is_dirty_when_versions_differ() {
        let p = Post::try_new(blog("s", "t", "body", vec![])).unwrap();
        let entry = ManifestEntry {
            post_version: "deadbeef".into(),
            chunk_count: 1,
            ingested_at: "2026-01-01".into(),
        };
        assert!(p.is_dirty(Some(&entry)));
    }

    #[test]
    fn not_dirty_when_versions_match() {
        let p = Post::try_new(blog("s", "t", "body", vec![])).unwrap();
        let entry = ManifestEntry {
            post_version: p.version().as_str().to_string(),
            chunk_count: 1,
            ingested_at: "2026-01-01".into(),
        };
        assert!(!p.is_dirty(Some(&entry)));
    }

    #[test]
    fn glossary_chunks_use_sequential_ids_from_start() {
        let p = Post::try_new(blog(
            "s",
            "t",
            "body",
            vec![term("a", "Alpha"), term("b", "Beta")],
        ))
        .unwrap();
        let chunks = p.glossary_chunks(5);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chunk_id, 5);
        assert_eq!(chunks[1].chunk_id, 6);
        assert!(chunks.iter().all(|c| c.is_glossary));
        assert!(chunks[0].heading.starts_with("Glossary:"));
    }

    #[test]
    fn version_short_is_eight_chars() {
        let p = Post::try_new(blog("s", "t", "body", vec![])).unwrap();
        assert_eq!(p.version().short().len(), 8);
    }

    #[test]
    fn metadata_for_glossary_chunk_includes_sources() {
        let p =
            Post::try_new(blog("s", "t", "body", vec![term("g", "G")])).unwrap();
        let chunks = p.glossary_chunks(0);
        let meta = p.metadata_for(&chunks[0]);
        assert!(meta.get("sources").is_some());
    }

    #[test]
    fn metadata_for_body_chunk_omits_sources() {
        let p = Post::try_new(blog("s", "t", "body", vec![])).unwrap();
        let chunk = Chunk {
            chunk_id: 0,
            heading: "h".into(),
            text: "t".into(),
            char_start: 0,
            char_end: 1,
            sources: vec![],
            is_glossary: false,
        };
        let meta = p.metadata_for(&chunk);
        assert!(meta.get("sources").is_none());
    }
}
