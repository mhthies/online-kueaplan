
--- general function for automatically setting the `last_updated` field with a trigger ---
CREATE FUNCTION sync_lastmod() RETURNS trigger AS $$
BEGIN
    NEW.last_updated := NOW();

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;


--- table events ---
CREATE TABLE events (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    begin_date DATE NOT NULL,
    end_date DATE NOT NULL
);


--- table categories ---
CREATE TABLE categories (
    id UUID PRIMARY KEY,
    title VARCHAR NOT NULL,
    icon VARCHAR NOT NULL,
    color CHAR(6) NOT NULL,
    event_id SERIAL REFERENCES events(id),
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    is_official BOOL NOT NULL DEFAULT FALSE,
    sort_key INT NOT NULL DEFAULT 0
);

CREATE INDEX ON categories (event_id, sort_key);

CREATE TRIGGER
    sync_lastmod
    BEFORE UPDATE ON categories
    FOR EACH ROW EXECUTE PROCEDURE sync_lastmod();


--- table rooms ---
CREATE TABLE rooms (
    id UUID PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    event_id SERIAL REFERENCES events(id),
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX ON rooms (event_id, title);

CREATE TRIGGER
    sync_lastmod
    BEFORE UPDATE ON rooms
    FOR EACH ROW EXECUTE PROCEDURE sync_lastmod();


--- table entries ---
CREATE TABLE entries (
    id UUID PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    responsible_person VARCHAR NOT NULL,
    is_room_reservation BOOLEAN NOT NULL DEFAULT FALSE,
    event_id SERIAL REFERENCES events(id),
    "begin" TIMESTAMP WITH TIME ZONE NOT NULL,
    "end" TIMESTAMP WITH TIME ZONE NOT NULL,
    category UUID REFERENCES categories(id) NOT NULL,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    "comment" VARCHAR NOT NULL DEFAULT '',
    time_comment VARCHAR NOT NULL DEFAULT '',
    room_comment VARCHAR NOT NULL DEFAULT '',
    is_exclusive BOOL NOT NULL DEFAULT FALSE,
    is_cancelled BOOL NOT NULL DEFAULT FALSE
);

CREATE INDEX ON entries (event_id, begin);

CREATE TRIGGER
    sync_lastmod
    BEFORE UPDATE ON entries
    FOR EACH ROW EXECUTE PROCEDURE sync_lastmod();


--- table entry_rooms ---
CREATE TABLE entry_rooms (
    entry_id UUID REFERENCES entries(id) ON DELETE CASCADE,
    room_id UUID REFERENCES rooms(id),

    PRIMARY KEY (entry_id, room_id)
);


--- table previous_dates ---
CREATE TABLE previous_dates (
    id UUID PRIMARY KEY,
    entry_id UUID NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    comment VARCHAR NOT NULL ,
    "begin" TIMESTAMP WITH TIME ZONE NOT NULL,
    "end" TIMESTAMP WITH TIME ZONE NOT NULL,
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX ON previous_dates(entry_id);

CREATE TRIGGER sync_lastmod
    BEFORE UPDATE ON previous_dates
    FOR EACH ROW EXECUTE PROCEDURE sync_lastmod();


--- table previous_date_rooms ---
CREATE TABLE previous_date_rooms (
    previous_date_id UUID REFERENCES previous_dates(id) ON DELETE CASCADE,
    room_id UUID REFERENCES rooms(id),

    PRIMARY KEY (previous_date_id, room_id)
);


--- table event_passphrases ---
CREATE TABLE event_passphrases (
    id SERIAL PRIMARY KEY,
    event_id SERIAL REFERENCES events(id),
    privilege INTEGER NOT NULL,
    passphrase VARCHAR NOT NULL
);

CREATE UNIQUE INDEX ON event_passphrases (event_id, passphrase);


--- table announcements ---
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
    BEFORE UPDATE ON announcements
    FOR EACH ROW EXECUTE PROCEDURE sync_lastmod();


--- table announcement_categories ---
CREATE TABLE announcement_categories (
    announcement_id UUID REFERENCES announcements(id) ON DELETE CASCADE,
    category_id UUID REFERENCES categories(id),

    PRIMARY KEY (announcement_id, category_id)
);

--- table announcement_rooms ---
CREATE TABLE announcement_rooms (
    announcement_id UUID REFERENCES announcements(id) ON DELETE CASCADE,
    room_id UUID REFERENCES rooms(id),

    PRIMARY KEY (announcement_id, room_id)
);
