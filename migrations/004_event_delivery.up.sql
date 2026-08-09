ALTER TABLE event_outbox
    ADD COLUMN sequence BIGINT;

UPDATE event_outbox
SET sequence = aggregate_version
WHERE sequence IS NULL;

ALTER TABLE event_outbox
    ALTER COLUMN sequence SET NOT NULL;

ALTER TABLE event_outbox
    DROP CONSTRAINT event_outbox_aggregate_type_aggregate_id_aggregate_version_key;

ALTER TABLE event_outbox
    ADD CONSTRAINT event_outbox_aggregate_sequence_key
    UNIQUE (aggregate_type, aggregate_id, sequence);

CREATE INDEX event_outbox_replay_idx
    ON event_outbox (organization_id, project_id, aggregate_id, sequence);

CREATE TABLE event_delivery (
    consumer_id TEXT NOT NULL,
    event_id TEXT NOT NULL REFERENCES event_outbox (event_id),
    delivered_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (consumer_id, event_id)
);

CREATE INDEX event_delivery_event_idx
    ON event_delivery (event_id);

CREATE OR REPLACE FUNCTION machina_begin_event_delivery(
    p_consumer_id TEXT,
    p_event_id TEXT,
    p_delivered_at TIMESTAMPTZ
)
RETURNS BOOLEAN
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO event_delivery (consumer_id, event_id, delivered_at)
    VALUES (p_consumer_id, p_event_id, p_delivered_at)
    ON CONFLICT (consumer_id, event_id) DO NOTHING;
    RETURN FOUND;
END;
$$;
