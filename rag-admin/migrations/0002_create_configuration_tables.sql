CREATE TABLE IF NOT EXISTS configuration (
    id UUID PRIMARY KEY,
    ai_providers JSONB NOT NULL DEFAULT '[]',
    vector_store_providers JSONB NOT NULL DEFAULT '[]',
    embedding_models JSONB NOT NULL DEFAULT '[]',
    generation_models JSONB NOT NULL DEFAULT '[]',
    vector_indexes JSONB NOT NULL DEFAULT '[]',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS pipeline_configurations (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    embedding_model_id UUID NOT NULL,
    generation_model_id UUID NOT NULL,
    vector_index_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);