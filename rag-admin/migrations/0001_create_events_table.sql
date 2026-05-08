CREATE TABLE IF NOT EXISTS events (
    id BIGSERIAL PRIMARY KEY,
    stream_id UUID NOT NULL,
    aggregate_type TEXT NOT NULL,
    position BIGINT NOT NULL,
    event_type TEXT NOT NULL,
    event_data JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT events_stream_position_unique UNIQUE (stream_id, position)
);

CREATE INDEX IF NOT EXISTS events_stream_id_idx ON events (stream_id, position);