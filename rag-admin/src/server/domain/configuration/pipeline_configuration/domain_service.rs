use crate::server::domain::configuration::{
    embedding_model::EmbeddingModel, exceptions::ConfigurationError, vector_index::VectorIndex,
};

pub struct PipelineConfigurationValidator;

impl PipelineConfigurationValidator {
    pub fn validate_combination(
        embedding_model: &EmbeddingModel,
        vector_index: &VectorIndex,
    ) -> Result<(), ConfigurationError> {
        if embedding_model.dimensions != vector_index.dimensions {
            return Err(ConfigurationError::ValidationError(format!(
                "embedding model dimensions ({}) do not match vector index dimensions ({})",
                embedding_model.dimensions, vector_index.dimensions
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn embedding_model(dimensions: u32) -> EmbeddingModel {
        EmbeddingModel {
            embedding_model_id: Uuid::new_v4(),
            provider_id: Uuid::new_v4(),
            model: "test-model".into(),
            dimensions,
        }
    }

    fn vector_index(dimensions: u32) -> VectorIndex {
        VectorIndex {
            index_id: Uuid::new_v4(),
            vector_store_provider_id: Uuid::new_v4(),
            name: "test-index".into(),
            dimensions,
        }
    }

    #[test]
    fn matching_dimensions_are_valid() {
        assert!(PipelineConfigurationValidator::validate_combination(
            &embedding_model(1536),
            &vector_index(1536)
        )
        .is_ok());
    }

    #[test]
    fn mismatched_dimensions_are_invalid() {
        let err = PipelineConfigurationValidator::validate_combination(
            &embedding_model(1536),
            &vector_index(1024),
        )
        .unwrap_err();
        assert!(matches!(err, ConfigurationError::ValidationError(_)));
    }
}
