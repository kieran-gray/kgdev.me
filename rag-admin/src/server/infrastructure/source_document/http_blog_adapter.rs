use std::sync::Arc;

use async_trait::async_trait;

use crate::server::application::{
    blog::ports::BlogSource,
    source_document::ports::source_adapter::{DocumentSummary, FetchedDocument, SourceAdapter},
    AppError,
};
use crate::server::domain::source_document::{
    document_type::DocumentType,
    source_ref::SourceRef,
    version::{BlogPostMetadata, DocumentMetadata},
};

pub struct HttpBlogAdapter {
    blog_source: Arc<dyn BlogSource>,
}

impl HttpBlogAdapter {
    pub fn new(blog_source: Arc<dyn BlogSource>) -> Arc<Self> {
        Arc::new(Self { blog_source })
    }
}

#[async_trait]
impl SourceAdapter for HttpBlogAdapter {
    fn document_type(&self) -> DocumentType {
        DocumentType::BlogPost
    }

    async fn list(&self) -> Result<Vec<DocumentSummary>, AppError> {
        let summaries = self.blog_source.list().await?;
        Ok(summaries
            .into_iter()
            .map(|s| DocumentSummary {
                source_ref: SourceRef::UpstreamSlug { slug: s.slug },
                title: s.title,
            })
            .collect())
    }

    async fn fetch(&self, source_ref: &SourceRef) -> Result<FetchedDocument, AppError> {
        let slug = match source_ref {
            SourceRef::UpstreamSlug { slug } => slug,
        };

        let blog_post = self.blog_source.fetch(slug).await?;

        let content = blog_post.source_markdown.as_bytes().to_vec();
        let metadata = DocumentMetadata::BlogPost(BlogPostMetadata {
            title: blog_post.title.clone(),
            published_at: blog_post.published_at.clone(),
        });

        Ok(FetchedDocument {
            source_ref: source_ref.clone(),
            content,
            metadata,
        })
    }
}
