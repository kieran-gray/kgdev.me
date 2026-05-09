use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfiguration {
    pub pipeline_configuration_id: Uuid,
    pub name: String,
    pub embedding_model_id: Uuid,
    pub generation_model_id: Uuid,
    pub vector_index_id: Uuid,
}

// TODO: The pipeline configurations need to be versioned.
// When we run Chunk and Index a document it we will store a
// pipeline_configuration_id so we know where it was indexed to
// and with what models.
// Could be as simple as replaying the events up to the indexed_at
// time.
