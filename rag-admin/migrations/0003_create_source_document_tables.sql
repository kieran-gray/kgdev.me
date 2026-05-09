CREATE TABLE IF NOT EXISTS source_documents (
    document_id UUID PRIMARY KEY,
    read_model  JSONB NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS source_document_blobs (
    content_hash TEXT PRIMARY KEY,
    bytes        BYTEA NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
