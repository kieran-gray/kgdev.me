use crate::server::domain::Aggregate;

use super::{aggregate::Indexing, events::IndexingEvent, read_model::IndexingReadModel};

pub struct IndexingProjector;

impl IndexingProjector {
    pub fn project(events: &[IndexingEvent]) -> Option<IndexingReadModel> {
        Indexing::from_events(events).map(|i| IndexingReadModel::from(&i))
    }
}
