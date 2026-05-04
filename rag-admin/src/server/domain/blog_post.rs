use serde::{Deserialize, Serialize};

use super::PostVersion;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogPostSummary {
    pub slug: String,
    pub title: String,
    pub published_at: String,
    pub post_version: PostVersion,
}

#[derive(Debug, Clone)]
pub struct BlogPost {
    pub slug: String,
    pub title: String,
    pub published_at: String,
    pub source_markdown: String,
    pub markdown_body: String,
    pub plain_text: String,
    pub glossary_terms: Vec<GlossaryTerm>,
}

/// Field order on `GlossaryTerm` and `GlossarySource` is load-bearing: this
/// struct is serialized to JSON when computing `post_version`, and matching
/// the JS object key insertion order keeps the hash stable across the two
/// implementations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryTerm {
    pub term: String,
    pub definition: String,
    pub sources: Vec<GlossarySource>,
    #[serde(skip)]
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossarySource {
    pub title: String,
    pub url: String,
}
