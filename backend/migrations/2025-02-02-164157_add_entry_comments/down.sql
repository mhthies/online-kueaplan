
ALTER TABLE entries
    DROP COLUMN "comment",
    DROP COLUMN time_comment,
    DROP COLUMN room_comment,
    DROP COLUMN is_exclusive,
    DROP COLUMN is_cancelled;

ALTER TABLE entries
    RENAME COLUMN is_room_reservation TO is_blocker;

ALTER TABLE categories
    DROP COLUMN is_official;
