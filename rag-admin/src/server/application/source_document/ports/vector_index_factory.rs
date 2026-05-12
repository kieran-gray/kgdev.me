use std::sync::Arc;

use crate::server::application::indexing::ports::VectorIndex;

/// One implementation per `VectorStoreKind`. Given an index *name* and
/// *dimensions*, build a concrete `VectorIndex` bound to that index.
///
/// Resolution from `vector_index_id` to (name, dims) lives in
/// `application::ingest::VectorIndexResolver` — this trait stays focused on
/// the per-backend wiring.
pub trait VectorIndexProvider: Send + Sync {
    fn build(&self, index_name: &str, dimensions: u32) -> Arc<dyn VectorIndex>;
}
