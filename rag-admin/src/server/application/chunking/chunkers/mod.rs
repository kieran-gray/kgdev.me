pub mod bert;
pub mod llm;
pub mod section;

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::server::application::chunking::ChunkerRegistry;
use crate::server::application::ports::ChatClient;
use crate::shared::SettingsDto;

pub use bert::BertChunker;
pub use llm::LlmChunker;
pub use section::SectionChunker;

pub struct BuiltinChunkerDeps {
    pub chat_client: Arc<dyn ChatClient>,
    pub settings: Arc<RwLock<SettingsDto>>,
}

pub fn register_builtin_chunkers(registry: &mut ChunkerRegistry, deps: BuiltinChunkerDeps) {
    registry.add(Arc::new(SectionChunker {}));
    registry.add(Arc::new(BertChunker {}));
    registry.add(Arc::new(LlmChunker::create(
        deps.chat_client,
        deps.settings,
    )));
}
