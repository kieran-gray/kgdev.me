CREATE TABLE IF NOT EXISTS indexings (
    indexing_id UUID PRIMARY KEY,
    document_id UUID NOT NULL,
    read_model   JSONB NOT NULL,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS indexings_document_id_idx ON indexings (document_id);

CREATE TABLE IF NOT EXISTS chunk_sets (
    chunk_set_id     UUID PRIMARY KEY,
    document_id      UUID NOT NULL,
    document_version INT  NOT NULL,
    chunking_config  JSONB NOT NULL,
    created_at       TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS chunk_sets_document_id_idx ON chunk_sets (document_id);

CREATE TABLE IF NOT EXISTS chunks (
    chunk_id     UUID PRIMARY KEY,
    chunk_set_id UUID NOT NULL REFERENCES chunk_sets (chunk_set_id) ON DELETE CASCADE,
    sequence     INT  NOT NULL,
    heading      TEXT NOT NULL,
    text         TEXT NOT NULL,
    char_start   INT  NOT NULL,
    char_end     INT  NOT NULL
);

CREATE INDEX IF NOT EXISTS chunks_chunk_set_id_idx ON chunks (chunk_set_id, sequence);

CREATE TABLE IF NOT EXISTS embedding_sets (
    embedding_set_id         UUID PRIMARY KEY,
    chunk_set_id             UUID NOT NULL REFERENCES chunk_sets (chunk_set_id) ON DELETE CASCADE,
    embedding_model_id       UUID NOT NULL,
    embedding_model_snapshot JSONB NOT NULL,
    dimensions               INT  NOT NULL,
    created_at               TEXT NOT NULL,
    CONSTRAINT embedding_sets_chunk_model_unique UNIQUE (chunk_set_id, embedding_model_id)
);

CREATE TABLE IF NOT EXISTS chunk_embeddings (
    chunk_id         UUID NOT NULL REFERENCES chunks (chunk_id) ON DELETE CASCADE,
    embedding_set_id UUID NOT NULL REFERENCES embedding_sets (embedding_set_id) ON DELETE CASCADE,
    vector           JSONB NOT NULL,
    PRIMARY KEY (chunk_id, embedding_set_id)
);
