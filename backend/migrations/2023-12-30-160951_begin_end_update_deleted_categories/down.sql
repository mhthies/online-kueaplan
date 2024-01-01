ALTER TABLE entries
    DROP "begin",
    DROP "end",
    DROP category,
    DROP deleted,
    DROP last_updated;


ALTER TABLE rooms
    DROP deleted,
    DROP last_updated;


DROP TABLE categories;

DROP TRIGGER sync_lastmod on entries;
DROP TRIGGER sync_lastmod on rooms;

DROP FUNCTION sync_lastmod;
