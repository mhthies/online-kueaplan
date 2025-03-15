
-- Add new structures for previous_dates
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

CREATE TABLE previous_date_rooms (
    previous_date_id UUID REFERENCES previous_dates(id) ON DELETE CASCADE,
    room_id UUID REFERENCES rooms(id),

    PRIMARY KEY (previous_date_id, room_id)
);
CREATE INDEX ON previous_date_rooms (previous_date_id);


-- Data Migration
INSERT INTO previous_dates
    SELECT id, residue_of, comment, "begin", "end"
        FROM entries WHERE residue_of is not null;

INSERT INTO previous_date_rooms
    SELECT entry_rooms.entry_id, entry_rooms.room_id
        FROM entry_rooms INNER JOIN entries ON entry_rooms.entry_id = entries.id AND entries.residue_of IS NOT NULL;

DELETE FROM entries WHERE residue_of IS NOT NULL;

-- Remove residue_of
ALTER TABLE entries
    DROP residue_of;

-- TODO trigger
