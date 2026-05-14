CREATE TABLE embedding_models (
    id          UUID PRIMARY KEY,
    kind        TEXT NOT NULL,
    model       TEXT NOT NULL,
    dimensions  INTEGER NOT NULL CHECK (dimensions > 0),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (kind, model)
);

CREATE TABLE generation_models (
    id          UUID PRIMARY KEY,
    kind        TEXT NOT NULL,
    model       TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (kind, model)
);

CREATE TABLE vector_indexes (
    id          UUID PRIMARY KEY,
    kind        TEXT NOT NULL,
    name        TEXT NOT NULL UNIQUE,
    dimensions  INTEGER NOT NULL CHECK (dimensions > 0),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

DROP TABLE configuration;

DROP TABLE pipeline_configurations;
CREATE TABLE pipeline_configurations (
    id                   UUID PRIMARY KEY,
    name                 TEXT NOT NULL UNIQUE,
    embedding_model_id   UUID NOT NULL REFERENCES embedding_models(id)  ON DELETE RESTRICT,
    generation_model_id  UUID NOT NULL REFERENCES generation_models(id) ON DELETE RESTRICT,
    vector_index_id      UUID NOT NULL REFERENCES vector_indexes(id)    ON DELETE RESTRICT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE OR REPLACE FUNCTION pipeline_configurations_check_dimensions()
RETURNS TRIGGER AS $$
DECLARE
    embedding_dim INTEGER;
    index_dim     INTEGER;
BEGIN
    SELECT dimensions INTO embedding_dim FROM embedding_models WHERE id = NEW.embedding_model_id;
    SELECT dimensions INTO index_dim     FROM vector_indexes   WHERE id = NEW.vector_index_id;
    IF embedding_dim IS NULL OR index_dim IS NULL THEN
        RAISE EXCEPTION 'referenced embedding model or vector index not found'
            USING ERRCODE = '23503';
    END IF;
    IF embedding_dim <> index_dim THEN
        RAISE EXCEPTION 'embedding model dimensions (%) do not match vector index dimensions (%)',
            embedding_dim, index_dim
            USING ERRCODE = 'P0001';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER pipeline_configurations_check_dimensions
    BEFORE INSERT OR UPDATE OF embedding_model_id, vector_index_id ON pipeline_configurations
    FOR EACH ROW
    EXECUTE FUNCTION pipeline_configurations_check_dimensions();

DROP TABLE chunking_configurations;
CREATE TABLE chunking_configurations (
    id                   UUID PRIMARY KEY,
    name                 TEXT NOT NULL UNIQUE,
    generation_model_id  UUID REFERENCES generation_models(id) ON DELETE RESTRICT,
    config               JSONB NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
