pub mod bert;
pub mod llm;
pub mod section;

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::server::application::chunking::ChunkingEngine;
use crate::server::application::ports::{ChatClient, MarkdownParser};
use crate::shared::SettingsDto;

pub use bert::BertChunker;
pub use llm::LlmChunker;
pub use section::SectionChunker;

pub struct BuiltinChunkerDeps {
    pub chat_client: Arc<dyn ChatClient>,
    pub markdown_parser: Arc<dyn MarkdownParser>,
    pub settings: Arc<RwLock<SettingsDto>>,
}

pub fn register_builtin_chunkers(engine: &mut ChunkingEngine, deps: BuiltinChunkerDeps) {
    engine.add(Arc::new(SectionChunker::new(deps.markdown_parser.clone())));
    engine.add(Arc::new(BertChunker::new(deps.markdown_parser.clone())));
    engine.add(Arc::new(LlmChunker::create(
        deps.chat_client,
        deps.settings,
        deps.markdown_parser,
    )));
}
