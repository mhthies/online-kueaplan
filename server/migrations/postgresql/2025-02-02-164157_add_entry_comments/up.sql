
ALTER TABLE entries
    ADD "comment" VARCHAR NOT NULL DEFAULT '',
    ADD time_comment VARCHAR NOT NULL DEFAULT '',
    ADD room_comment VARCHAR NOT NULL DEFAULT '',
    ADD is_exclusive BOOL NOT NULL DEFAULT FALSE,
    ADD is_cancelled BOOL NOT NULL DEFAULT FALSE;

ALTER TABLE entries
    RENAME COLUMN is_blocker TO is_room_reservation;

ALTER TABLE categories
    ADD COLUMN is_official BOOL NOT NULL DEFAULT FALSE;
