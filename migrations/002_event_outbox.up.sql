CREATE TABLE event_outbox (
    event_id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations (organization_id),
    project_id TEXT,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    aggregate_version BIGINT NOT NULL,
    event_type TEXT NOT NULL,
    payload_json JSONB NOT NULL,
    classification TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    published_at TIMESTAMPTZ,
    UNIQUE (aggregate_type, aggregate_id, aggregate_version)
);

CREATE INDEX event_outbox_unpublished_idx
    ON event_outbox (created_at)
    WHERE published_at IS NULL;
