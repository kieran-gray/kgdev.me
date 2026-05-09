use std::collections::HashMap;

use uuid::Uuid;

use crate::server::domain::configuration::events::ConfigurationEvent;

use super::read_model::PipelineConfigurationReadModel;

pub struct PipelineConfigurationProjector;

impl PipelineConfigurationProjector {
    pub fn from_events(events: &[ConfigurationEvent]) -> Vec<PipelineConfigurationReadModel> {
        let mut map: HashMap<Uuid, PipelineConfigurationReadModel> = HashMap::new();

        for event in events {
            match event {
                ConfigurationEvent::PipelineConfigurationCreated(e) => {
                    map.insert(
                        e.pipeline_configuration_id,
                        PipelineConfigurationReadModel {
                            pipeline_configuration_id: e.pipeline_configuration_id,
                            name: e.name.clone(),
                            embedding_model_id: e.embedding_model_id,
                            generation_model_id: e.generation_model_id,
                            vector_index_id: e.vector_index_id,
                        },
                    );
                }
                ConfigurationEvent::PipelineConfigurationUpdated(e) => {
                    map.insert(
                        e.pipeline_configuration_id,
                        PipelineConfigurationReadModel {
                            pipeline_configuration_id: e.pipeline_configuration_id,
                            name: e.name.clone(),
                            embedding_model_id: e.embedding_model_id,
                            generation_model_id: e.generation_model_id,
                            vector_index_id: e.vector_index_id,
                        },
                    );
                }
                ConfigurationEvent::PipelineConfigurationDeleted(e) => {
                    map.remove(&e.pipeline_configuration_id);
                }
                _ => {}
            }
        }

        map.into_values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::domain::configuration::{events::ConfigurationCreated, pipeline_configuration::events::{PipelineConfigurationCreated, PipelineConfigurationDeleted, PipelineConfigurationUpdated}};
    use uuid::Uuid;

    #[test]
    fn projects_created_configurations() {
        let id = Uuid::new_v4();
        let result = PipelineConfigurationProjector::from_events(&[
            ConfigurationEvent::ConfigurationCreated(ConfigurationCreated {
                configuration_id: Uuid::nil(),
            }),
            ConfigurationEvent::PipelineConfigurationCreated(PipelineConfigurationCreated {
                pipeline_configuration_id: id,
                name: "production".into(),
                embedding_model_id: Uuid::new_v4(),
                generation_model_id: Uuid::new_v4(),
                vector_index_id: Uuid::new_v4(),
            }),
        ]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "production");
    }

    #[test]
    fn projects_updated_configuration() {
        let id = Uuid::new_v4();
        let new_index_id = Uuid::new_v4();
        let result = PipelineConfigurationProjector::from_events(&[
            ConfigurationEvent::PipelineConfigurationCreated(PipelineConfigurationCreated {
                pipeline_configuration_id: id,
                name: "production".into(),
                embedding_model_id: Uuid::new_v4(),
                generation_model_id: Uuid::new_v4(),
                vector_index_id: Uuid::new_v4(),
            }),
            ConfigurationEvent::PipelineConfigurationUpdated(PipelineConfigurationUpdated {
                pipeline_configuration_id: id,
                name: "production".into(),
                embedding_model_id: Uuid::new_v4(),
                generation_model_id: Uuid::new_v4(),
                vector_index_id: new_index_id,
            }),
        ]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].vector_index_id, new_index_id);
    }

    #[test]
    fn projects_deleted_configuration() {
        let id = Uuid::new_v4();
        let result = PipelineConfigurationProjector::from_events(&[
            ConfigurationEvent::PipelineConfigurationCreated(PipelineConfigurationCreated {
                pipeline_configuration_id: id,
                name: "production".into(),
                embedding_model_id: Uuid::new_v4(),
                generation_model_id: Uuid::new_v4(),
                vector_index_id: Uuid::new_v4(),
            }),
            ConfigurationEvent::PipelineConfigurationDeleted(PipelineConfigurationDeleted {
                pipeline_configuration_id: id,
            }),
        ]);

        assert!(result.is_empty());
    }
}
