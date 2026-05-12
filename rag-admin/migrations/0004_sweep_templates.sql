CREATE TABLE sweep_templates (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    members JSONB NOT NULL DEFAULT '[]',
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX sweep_templates_one_default
    ON sweep_templates (is_default)
    WHERE is_default;
