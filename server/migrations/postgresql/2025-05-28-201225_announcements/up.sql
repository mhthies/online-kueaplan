CREATE TABLE announcements (
    id UUID PRIMARY KEY,
    event_id SERIAL REFERENCES events (id),
    announcement_type INT NOT NULL,
    text VARCHAR NOT NULL,
    show_with_days BOOL NOT NULL,
    begin_date DATE NULL,
    end_date DATE NULL,
    show_with_categories BOOL NOT NULL,
    show_with_all_categories BOOL NOT NULL,
    show_with_rooms BOOL NOT NULL,
    show_with_all_rooms BOOL NOT NULL,
    sort_key INT NOT NULL DEFAULT 0,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
CREATE INDEX ON announcements (event_id, sort_key);

CREATE TRIGGER
    sync_lastmod
    BEFORE UPDATE ON
    announcements
    FOR EACH ROW EXECUTE PROCEDURE
    sync_lastmod();

CREATE TABLE announcement_categories (
    announcement_id UUID REFERENCES announcements(id) ON DELETE CASCADE,
    category_id UUID REFERENCES categories(id),

    PRIMARY KEY (announcement_id, category_id)
);

CREATE TABLE announcement_rooms (
    announcement_id UUID REFERENCES announcements(id) ON DELETE CASCADE,
    room_id UUID REFERENCES rooms(id),

    PRIMARY KEY (announcement_id, room_id)
);
