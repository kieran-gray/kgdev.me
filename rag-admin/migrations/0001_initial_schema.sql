-- ───────────────────────────────────────────────────────────────────────────
-- Event sourcing core
-- ───────────────────────────────────────────────────────────────────────────

CREATE TABLE events (
    id BIGSERIAL PRIMARY KEY,
    stream_id UUID NOT NULL,
    aggregate_type TEXT NOT NULL,
    position BIGINT NOT NULL,
    event_type TEXT NOT NULL,
    event_data JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT events_stream_position_unique UNIQUE (stream_id, position)
);

CREATE INDEX events_stream_id_idx ON events (stream_id, position);
CREATE INDEX events_aggregate_type_id_idx ON events (aggregate_type, id);

CREATE TABLE aggregate_snapshots (
    stream_id UUID PRIMARY KEY,
    aggregate_type TEXT NOT NULL,
    version BIGINT NOT NULL,
    snapshot JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE projection_checkpoints (
    projector_name TEXT PRIMARY KEY,
    last_processed_log_position BIGINT NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'healthy',
    error_message TEXT,
    error_count BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE pending_effects (
    effect_id UUID PRIMARY KEY,
    aggregate_type TEXT NOT NULL,
    stream_id UUID NOT NULL,
    event_log_position BIGINT NOT NULL,
    effect_type TEXT NOT NULL,
    effect_payload JSONB NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'pending',
    attempts INT NOT NULL DEFAULT 0,
    last_attempt_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX pending_effects_pending_idx
    ON pending_effects (aggregate_type, status, attempts)
    WHERE status IN ('pending', 'failed');

CREATE OR REPLACE FUNCTION notify_events_appended() RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('events_appended', NEW.aggregate_type);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER events_appended_trigger
    AFTER INSERT ON events
    FOR EACH ROW EXECUTE FUNCTION notify_events_appended();

-- ───────────────────────────────────────────────────────────────────────────
-- Configuration read model
-- ───────────────────────────────────────────────────────────────────────────
-- The configuration aggregate (singleton, id = uuid nil) is projected into a
-- single JSONB-wide row. Each embedding/generation model and vector index
-- carries its `kind` (AiProviderKind / VectorStoreKind) directly — there are
-- no separate provider entities.

CREATE TABLE configuration (
    id UUID PRIMARY KEY,
    embedding_models  JSONB NOT NULL DEFAULT '[]',
    generation_models JSONB NOT NULL DEFAULT '[]',
    vector_indexes    JSONB NOT NULL DEFAULT '[]',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE pipeline_configurations (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    embedding_model_id UUID NOT NULL,
    generation_model_id UUID NOT NULL,
    vector_index_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE chunking_configurations (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    config JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ───────────────────────────────────────────────────────────────────────────
-- Source document read model
-- ───────────────────────────────────────────────────────────────────────────

CREATE TABLE source_documents (
    document_id UUID PRIMARY KEY,
    document_type TEXT NOT NULL,
    source_ref JSONB NOT NULL,
    latest_version_number INT NOT NULL,
    latest_content_hash TEXT NOT NULL,
    latest_metadata JSONB NOT NULL,
    latest_version_occurred_at TEXT NOT NULL,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX source_documents_source_ref_idx ON source_documents USING GIN (source_ref);

CREATE TABLE source_document_blobs (
    content_hash TEXT PRIMARY KEY,
    bytes BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ───────────────────────────────────────────────────────────────────────────
-- Chunking / embedding / indexing
-- ───────────────────────────────────────────────────────────────────────────

CREATE TABLE indexings (
    indexing_id UUID PRIMARY KEY,
    document_id UUID NOT NULL,
    pipeline_configuration_id UUID NOT NULL,
    document_version INT NOT NULL,
    chunking_config JSONB NOT NULL,
    chunk_set_id UUID,
    embedding_set_id UUID,
    status TEXT NOT NULL,
    failure_stage TEXT,
    attempts INT NOT NULL,
    removed BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX indexings_document_id_idx ON indexings (document_id);

CREATE TABLE chunk_sets (
    chunk_set_id UUID PRIMARY KEY,
    document_id UUID NOT NULL,
    document_version INT NOT NULL,
    chunking_config JSONB NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX chunk_sets_document_id_idx ON chunk_sets (document_id);

CREATE TABLE chunks (
    chunk_id UUID PRIMARY KEY,
    chunk_set_id UUID NOT NULL REFERENCES chunk_sets (chunk_set_id) ON DELETE CASCADE,
    sequence INT NOT NULL,
    heading TEXT NOT NULL,
    text TEXT NOT NULL,
    char_start INT NOT NULL,
    char_end INT NOT NULL
);
CREATE INDEX chunks_chunk_set_id_idx ON chunks (chunk_set_id, sequence);

CREATE TABLE embedding_sets (
    embedding_set_id UUID PRIMARY KEY,
    chunk_set_id UUID NOT NULL REFERENCES chunk_sets (chunk_set_id) ON DELETE CASCADE,
    embedding_model_id UUID NOT NULL,
    embedding_model_snapshot JSONB NOT NULL,
    dimensions INT NOT NULL,
    created_at TEXT NOT NULL,
    CONSTRAINT embedding_sets_chunk_model_unique UNIQUE (chunk_set_id, embedding_model_id)
);

CREATE TABLE chunk_embeddings (
    chunk_id UUID NOT NULL REFERENCES chunks (chunk_id) ON DELETE CASCADE,
    embedding_set_id UUID NOT NULL REFERENCES embedding_sets (embedding_set_id) ON DELETE CASCADE,
    vector JSONB NOT NULL,
    PRIMARY KEY (chunk_id, embedding_set_id)
);

-- ───────────────────────────────────────────────────────────────────────────
-- Evaluation
-- ───────────────────────────────────────────────────────────────────────────

CREATE TABLE evaluation_datasets (
    dataset_id UUID PRIMARY KEY,
    document_id UUID NOT NULL,
    document_version INT NOT NULL,
    content_hash TEXT NOT NULL,
    label TEXT NOT NULL,
    target_question_count INT NOT NULL,
    generation_model_id UUID NOT NULL,
    generation_model TEXT NOT NULL,
    excerpt_similarity_threshold_milli INT NOT NULL,
    duplicate_similarity_threshold_milli INT NOT NULL,
    embedding_model_id UUID NOT NULL,
    status TEXT NOT NULL,
    question_count INT NOT NULL,
    rejection_count INT NOT NULL,
    failure_reason TEXT,
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX evaluation_datasets_document_id_idx ON evaluation_datasets (document_id);
CREATE INDEX evaluation_datasets_deleted_at_idx
    ON evaluation_datasets (deleted_at)
    WHERE deleted_at IS NULL;

CREATE TABLE evaluation_questions (
    dataset_id UUID NOT NULL REFERENCES evaluation_datasets (dataset_id) ON DELETE CASCADE,
    sequence INT NOT NULL,
    question TEXT NOT NULL,
    embedding JSONB,
    PRIMARY KEY (dataset_id, sequence)
);

CREATE TABLE evaluation_references (
    dataset_id UUID NOT NULL,
    question_sequence INT NOT NULL,
    sequence INT NOT NULL,
    content TEXT NOT NULL,
    char_start INT NOT NULL,
    char_end INT NOT NULL,
    embedding JSONB,
    PRIMARY KEY (dataset_id, question_sequence, sequence),
    FOREIGN KEY (dataset_id, question_sequence)
        REFERENCES evaluation_questions (dataset_id, sequence) ON DELETE CASCADE
);

CREATE TABLE evaluation_runs (
    run_id UUID PRIMARY KEY,
    dataset_id UUID NOT NULL REFERENCES evaluation_datasets (dataset_id),
    pipeline_configuration_id UUID NOT NULL,
    document_id UUID NOT NULL,
    document_version INT NOT NULL,
    variants JSONB NOT NULL,
    options JSONB NOT NULL,
    autotune_request JSONB,
    status TEXT NOT NULL,
    variants_count INT NOT NULL,
    variants_prepared INT NOT NULL,
    variants_scored INT NOT NULL,
    failure_reason TEXT,
    scoring_recall_weight REAL NOT NULL,
    scoring_iou_weight REAL NOT NULL,
    scoring_precision_weight REAL NOT NULL,
    scoring_precision_omega_weight REAL NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX evaluation_runs_document_id_idx ON evaluation_runs (document_id);
CREATE INDEX evaluation_runs_dataset_id_idx ON evaluation_runs (dataset_id);

CREATE TABLE evaluation_variant_results (
    run_id UUID NOT NULL REFERENCES evaluation_runs (run_id) ON DELETE CASCADE,
    variant_label TEXT NOT NULL,
    split TEXT NOT NULL,
    variant_config JSONB NOT NULL,
    options JSONB NOT NULL,
    recall_mean REAL NOT NULL,
    recall_std REAL NOT NULL,
    precision_mean REAL NOT NULL,
    precision_std REAL NOT NULL,
    iou_mean REAL NOT NULL,
    iou_std REAL NOT NULL,
    precision_omega_mean REAL NOT NULL,
    precision_omega_std REAL NOT NULL,
    chunk_set_id UUID NOT NULL,
    embedding_set_id UUID NOT NULL,
    selected BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (run_id, variant_label, split)
);

CREATE TABLE retrieval_traces (
    run_id UUID NOT NULL,
    variant_label TEXT NOT NULL,
    split TEXT NOT NULL,
    question_sequence INT NOT NULL,
    retrieved_chunk_ids JSONB NOT NULL,
    scores JSONB NOT NULL,
    recall REAL NOT NULL,
    precision REAL NOT NULL,
    iou REAL NOT NULL,
    PRIMARY KEY (run_id, variant_label, split, question_sequence),
    FOREIGN KEY (run_id, variant_label, split)
        REFERENCES evaluation_variant_results (run_id, variant_label, split) ON DELETE CASCADE
);
