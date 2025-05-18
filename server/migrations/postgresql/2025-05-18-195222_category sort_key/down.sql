ALTER TABLE categories
    DROP COLUMN sort_key;
CREATE INDEX ON categories(event_id);

DROP INDEX rooms_event_id_title_idx;
CREATE INDEX ON rooms(event_id);

DROP INDEX entries_event_id_begin_idx;
CREATE INDEX ON entries(event_id);
