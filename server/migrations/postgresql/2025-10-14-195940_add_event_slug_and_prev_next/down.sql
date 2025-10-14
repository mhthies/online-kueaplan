DROP INDEX events_slug_idx;

ALTER TABLE events
    DROP COLUMN slug,
    DROP COLUMN preceding_event_id,
    DROP COLUMN subsequent_event_id;
