-- Production-like vector store backed by pgvector. Mirrors the Cloudflare
-- Vectorize VectorIndex surface so a pipeline can target either kind.
CREATE TABLE vector_index_records (
    index_name TEXT NOT NULL,
    id TEXT NOT NULL,
    vec VECTOR NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    PRIMARY KEY (index_name, id)
);

CREATE INDEX vector_index_records_index_name_idx
    ON vector_index_records (index_name);
