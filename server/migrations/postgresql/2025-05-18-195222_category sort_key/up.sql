ALTER TABLE categories
    ADD COLUMN sort_key INT NOT NULL DEFAULT 0;
CREATE INDEX ON categories (event_id, sort_key);
DROP INDEX categories_event_id_idx;

CREATE INDEX ON rooms (event_id, title);
DROP INDEX rooms_event_id_idx;

CREATE INDEX ON entries (event_id, begin);
DROP INDEX entries_event_id_idx;