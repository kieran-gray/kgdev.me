pub mod bert;
pub mod llm;
pub mod section;

use std::sync::Arc;

use crate::server::application::chunking::ChunkerRegistry;
use crate::server::application::ports::ChatClient;
use crate::server::domain::configuration::aggregate::Configuration;
use crate::server::event_sourcing::AggregateRepository;

pub use bert::BertChunker;
pub use llm::LlmChunker;
pub use section::SectionChunker;

pub struct BuiltinChunkerDeps {
    pub chat_client: Arc<dyn ChatClient>,
    pub configuration_repository: Arc<AggregateRepository<Configuration>>,
}

pub fn register_builtin_chunkers(registry: &mut ChunkerRegistry, deps: BuiltinChunkerDeps) {
    registry.add(Arc::new(SectionChunker {}));
    registry.add(Arc::new(BertChunker {}));
    registry.add(Arc::new(LlmChunker::create(
        deps.chat_client,
        deps.configuration_repository,
    )));
}
