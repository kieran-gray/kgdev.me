use std::sync::Arc;

use crate::server::application::ingest::ports::VectorIndex;

pub trait VectorIndexFactory: Send + Sync {
    fn for_index(&self, index_name: &str) -> Arc<dyn VectorIndex>;
}
