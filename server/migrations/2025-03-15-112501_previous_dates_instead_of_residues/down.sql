-- Re-add residue_of
ALTER TABLE entries
    ADD residue_of UUID REFERENCES entries (id);

-- Data Migration
INSERT INTO entries (id, title, description, responsible_person, residue_of, event_id, "begin", "end", category,
                     deleted, last_updated, comment, is_cancelled)
SELECT previous_dates.id,
       entries.title,
       '',
       entries.responsible_person,
       entries.id,
       entries.event_id,
       previous_dates."begin",
       previous_dates."end",
       entries.category,
       entries.deleted,
       entries.last_updated,
       previous_dates.comment,
       entries.is_cancelled
FROM previous_dates
         INNER JOIN entries ON previous_dates.entry_id = entries.id;

INSERT INTO entry_rooms
SELECT previous_date_id, room_id
    FROM previous_date_rooms;

-- Drop new tables
DROP TABLE previous_date_rooms;
DROP TABLE previous_dates;
