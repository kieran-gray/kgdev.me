CREATE TABLE IF NOT EXISTS pipeline_configuration (
    id UUID PRIMARY KEY,
    ai_providers JSONB NOT NULL DEFAULT '[]',
    vector_store_providers JSONB NOT NULL DEFAULT '[]',
    embedding_models JSONB NOT NULL DEFAULT '[]',
    generation_models JSONB NOT NULL DEFAULT '[]',
    vector_indexes JSONB NOT NULL DEFAULT '[]',
    current_embedding_model_id UUID,
    current_generation_model_id UUID,
    current_vector_index_id UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);