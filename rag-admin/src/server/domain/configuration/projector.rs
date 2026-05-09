use super::{
    aggregate::Configuration, events::ConfigurationEvent, read_model::ConfigurationReadModel,
};
use crate::server::domain::Aggregate;

pub struct ConfigurationProjector;

impl ConfigurationProjector {
    pub fn project(events: &[ConfigurationEvent]) -> ConfigurationReadModel {
        Configuration::from_events(events)
            .map(|c| ConfigurationReadModel::from(&c))
            .unwrap_or_default()
    }
}
