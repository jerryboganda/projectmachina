DROP FUNCTION IF EXISTS machina_begin_event_delivery(TEXT, TEXT, TIMESTAMPTZ);
DROP TABLE IF EXISTS event_delivery;

DROP INDEX IF EXISTS event_outbox_replay_idx;
ALTER TABLE event_outbox
    DROP CONSTRAINT IF EXISTS event_outbox_aggregate_sequence_key;
ALTER TABLE event_outbox
    DROP COLUMN IF EXISTS sequence;
ALTER TABLE event_outbox
    ADD CONSTRAINT event_outbox_aggregate_type_aggregate_id_aggregate_version_key
    UNIQUE (aggregate_type, aggregate_id, aggregate_version);
