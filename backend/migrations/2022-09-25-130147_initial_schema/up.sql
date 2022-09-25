CREATE TABLE events (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    beginDate DATE NOT NULL,
    endDate DATE NOT NULL
);

CREATE TABLE entries (
    id UUID PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    responsiblePerson VARCHAR NOT NULL,
    isBlocker BOOLEAN NOT NULL DEFAULT FALSE,
    residueOf UUID REFERENCES entries (id),
    eventId SERIAL REFERENCES events(id)
);
CREATE INDEX ON entries (eventId);

CREATE TABLE rooms (
    id UUID PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    eventId SERIAL REFERENCES events(id)
);
CREATE INDEX ON entries (eventId);

CREATE TABLE entry_rooms (
    entryId UUID REFERENCES entries(id),
    roomId UUID REFERENCES rooms(id),
    
    PRIMARY KEY (entryId, roomId)
);
CREATE INDEX ON entry_rooms (entryId);
