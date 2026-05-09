use super::{
    aggregate::SourceDocument, events::SourceDocumentEvent, read_model::SourceDocumentReadModel,
};
use crate::server::domain::Aggregate;

pub struct SourceDocumentProjector;

impl SourceDocumentProjector {
    pub fn project(events: &[SourceDocumentEvent]) -> Option<SourceDocumentReadModel> {
        SourceDocument::from_events(events).map(|doc| SourceDocumentReadModel::from(&doc))
    }
}
