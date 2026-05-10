-- migrations/0006_event_sourcing_infrastructure.sql
--
-- Adds the cross-cutting event sourcing infrastructure:
--   * an aggregate snapshot table so command handling does not replay full streams,
--   * per-projector checkpoints with health/error counts,
--   * an effect ledger for the process manager,
--   * a NOTIFY trigger so projection drivers wake up immediately on append.
--
-- The events table already exposes a global monotonically-increasing position
-- via its `id BIGSERIAL PRIMARY KEY` column; the EventStore adapter reads that
-- column as `log_position`. No ALTER TABLE is required here.

CREATE INDEX IF NOT EXISTS events_aggregate_type_id_idx
    ON events (aggregate_type, id);

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
