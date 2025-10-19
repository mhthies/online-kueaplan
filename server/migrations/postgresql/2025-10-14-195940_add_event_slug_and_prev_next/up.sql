ALTER TABLE events
    ADD COLUMN slug VARCHAR NULL,
    ADD COLUMN preceding_event_id INTEGER NULL REFERENCES events(id),
    ADD COLUMN subsequent_event_id INTEGER NULL REFERENCES events(id);

CREATE INDEX ON events(slug);
