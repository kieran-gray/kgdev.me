use std::sync::Arc;

use crate::server::application::ingest::ports::VectorIndex;
use crate::server::application::source_document::ports::VectorIndexFactory;
use crate::server::infrastructure::clients::CloudflareApi;

use super::named_vectorize_index::NamedVectorizeIndex;

pub struct CloudflareVectorIndexFactory {
    api: Arc<CloudflareApi>,
}

impl CloudflareVectorIndexFactory {
    pub fn new(api: Arc<CloudflareApi>) -> Arc<Self> {
        Arc::new(Self { api })
    }
}

impl VectorIndexFactory for CloudflareVectorIndexFactory {
    fn for_index(&self, index_name: &str) -> Arc<dyn VectorIndex> {
        NamedVectorizeIndex::new(self.api.clone(), index_name.to_string())
    }
}
