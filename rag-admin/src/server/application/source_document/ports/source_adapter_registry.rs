use std::collections::HashMap;
use std::sync::Arc;

use crate::server::domain::source_document::document_type::DocumentType;

use super::source_adapter::SourceAdapter;

pub struct SourceAdapterRegistry {
    adapters: HashMap<String, Arc<dyn SourceAdapter>>,
}

impl SourceAdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    pub fn register(&mut self, adapter: Arc<dyn SourceAdapter>) {
        let key = format!("{:?}", adapter.document_type());
        self.adapters.insert(key, adapter);
    }

    pub fn get(&self, document_type: &DocumentType) -> Option<&Arc<dyn SourceAdapter>> {
        let key = format!("{:?}", document_type);
        self.adapters.get(&key)
    }
}

impl Default for SourceAdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
