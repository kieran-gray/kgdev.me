use std::collections::HashMap;
use std::sync::Arc;

use uuid::Uuid;

use crate::server::application::ingest::ports::VectorIndex;
use crate::server::application::source_document::ports::VectorIndexProvider;
use crate::server::application::AppError;
use crate::server::domain::configuration::aggregate::Configuration;
use crate::server::domain::configuration::kinds::VectorStoreKind;
use crate::server::event_sourcing::{Aggregate, AggregateRepository};

#[derive(Debug, Clone)]
pub struct ResolvedVectorIndex {
    pub index_id: Uuid,
    pub kind: VectorStoreKind,
    pub name: String,
    pub dimensions: u32,
}

pub struct VectorIndexResolver {
    providers: HashMap<VectorStoreKind, Arc<dyn VectorIndexProvider>>,
    configuration_repository: Arc<AggregateRepository<Configuration>>,
}

impl VectorIndexResolver {
    pub fn new(
        providers: HashMap<VectorStoreKind, Arc<dyn VectorIndexProvider>>,
        configuration_repository: Arc<AggregateRepository<Configuration>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            providers,
            configuration_repository,
        })
    }

    pub async fn for_index_id(&self, index_id: Uuid) -> Result<Arc<dyn VectorIndex>, AppError> {
        let resolved = self.resolve(index_id).await?;
        self.build(&resolved)
    }

    pub fn build(&self, resolved: &ResolvedVectorIndex) -> Result<Arc<dyn VectorIndex>, AppError> {
        let provider = self.providers.get(&resolved.kind).ok_or_else(|| {
            AppError::Internal(format!(
                "no vector index provider registered for kind {}",
                resolved.kind.as_str()
            ))
        })?;
        Ok(provider.build(&resolved.name, resolved.dimensions))
    }

    pub async fn resolve(&self, index_id: Uuid) -> Result<ResolvedVectorIndex, AppError> {
        let Some(loaded) = self
            .configuration_repository
            .load(Configuration::singleton_id())
            .await?
        else {
            return Err(AppError::NotFound(
                Configuration::aggregate_type().to_string(),
            ));
        };

        let index = loaded
            .aggregate
            .vector_indexes
            .iter()
            .find(|i| i.index_id == index_id)
            .ok_or_else(|| {
                AppError::NotFound(format!("vector index {index_id} not registered"))
            })?;
        Ok(ResolvedVectorIndex {
            index_id: index.index_id,
            kind: index.kind,
            name: index.name.clone(),
            dimensions: index.dimensions,
        })
    }
}
