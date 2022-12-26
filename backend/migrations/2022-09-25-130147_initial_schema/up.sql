CREATE TABLE events (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    begin_date DATE NOT NULL,
    end_date DATE NOT NULL
);

CREATE TABLE entries (
    id UUID PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    responsible_person VARCHAR NOT NULL,
    is_blocker BOOLEAN NOT NULL DEFAULT FALSE,
    residue_of UUID REFERENCES entries (id),
    event_id SERIAL REFERENCES events(id)
);
CREATE INDEX ON entries (event_id);

CREATE TABLE rooms (
    id UUID PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    event_id SERIAL REFERENCES events(id)
);
CREATE INDEX ON entries (event_id);

CREATE TABLE entry_rooms (
    entry_id UUID REFERENCES entries(id) ON DELETE CASCADE,
    room_id UUID REFERENCES rooms(id),
    
    PRIMARY KEY (entry_id, room_id)
);
CREATE INDEX ON entry_rooms (entry_id);
