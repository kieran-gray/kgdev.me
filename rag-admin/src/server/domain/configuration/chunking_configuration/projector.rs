use std::collections::HashMap;

use uuid::Uuid;

use crate::server::domain::configuration::events::ConfigurationEvent;

use super::read_model::ChunkingConfigurationReadModel;

pub struct ChunkingConfigurationProjector;

impl ChunkingConfigurationProjector {
    pub fn from_events(events: &[ConfigurationEvent]) -> Vec<ChunkingConfigurationReadModel> {
        let mut map: HashMap<Uuid, ChunkingConfigurationReadModel> = HashMap::new();

        for event in events {
            match event {
                ConfigurationEvent::ChunkingConfigurationCreated(e) => {
                    map.insert(
                        e.chunking_configuration_id,
                        ChunkingConfigurationReadModel {
                            chunking_configuration_id: e.chunking_configuration_id,
                            name: e.name.clone(),
                            config: e.config.clone(),
                        },
                    );
                }
                ConfigurationEvent::ChunkingConfigurationUpdated(e) => {
                    map.insert(
                        e.chunking_configuration_id,
                        ChunkingConfigurationReadModel {
                            chunking_configuration_id: e.chunking_configuration_id,
                            name: e.name.clone(),
                            config: e.config.clone(),
                        },
                    );
                }
                ConfigurationEvent::ChunkingConfigurationDeleted(e) => {
                    map.remove(&e.chunking_configuration_id);
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
    use crate::server::domain::configuration::chunking_configuration::events::{
        ChunkingConfigurationCreated, ChunkingConfigurationDeleted, ChunkingConfigurationUpdated,
    };
    use crate::shared::{ChunkingConfig, SectionChunkingConfig};
    use uuid::Uuid;

    #[test]
    fn projects_created_configurations() {
        let id = Uuid::new_v4();
        let result = ChunkingConfigurationProjector::from_events(&[
            ConfigurationEvent::ChunkingConfigurationCreated(ChunkingConfigurationCreated {
                chunking_configuration_id: id,
                name: "default".into(),
                config: ChunkingConfig::Section(SectionChunkingConfig::default()),
            }),
        ]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "default");
    }

    #[test]
    fn projects_updated_configuration() {
        let id = Uuid::new_v4();
        let result = ChunkingConfigurationProjector::from_events(&[
            ConfigurationEvent::ChunkingConfigurationCreated(ChunkingConfigurationCreated {
                chunking_configuration_id: id,
                name: "default".into(),
                config: ChunkingConfig::Section(SectionChunkingConfig {
                    max_section_tokens: 256,
                }),
            }),
            ConfigurationEvent::ChunkingConfigurationUpdated(ChunkingConfigurationUpdated {
                chunking_configuration_id: id,
                name: "default".into(),
                config: ChunkingConfig::Section(SectionChunkingConfig {
                    max_section_tokens: 512,
                }),
            }),
        ]);

        assert_eq!(result.len(), 1);
        if let ChunkingConfig::Section(c) = &result[0].config {
            assert_eq!(c.max_section_tokens, 512);
        } else {
            panic!("expected section");
        }
    }

    #[test]
    fn projects_deleted_configuration() {
        let id = Uuid::new_v4();
        let result = ChunkingConfigurationProjector::from_events(&[
            ConfigurationEvent::ChunkingConfigurationCreated(ChunkingConfigurationCreated {
                chunking_configuration_id: id,
                name: "default".into(),
                config: ChunkingConfig::Section(SectionChunkingConfig::default()),
            }),
            ConfigurationEvent::ChunkingConfigurationDeleted(ChunkingConfigurationDeleted {
                chunking_configuration_id: id,
            }),
        ]);

        assert!(result.is_empty());
    }
}
