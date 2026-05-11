#[derive(Debug, Clone)]
pub struct BlogPostSummary {
    pub slug: String,
    pub title: String,
    pub published_at: String,
}

#[derive(Debug, Clone)]
pub struct BlogPost {
    pub title: String,
    pub published_at: String,
    pub source_markdown: String,
}
