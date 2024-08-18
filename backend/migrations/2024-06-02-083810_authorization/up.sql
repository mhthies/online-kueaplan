
CREATE TABLE event_passphrases (
    id SERIAL PRIMARY KEY,
    event_id SERIAL REFERENCES events(id),
    privilege INTEGER NOT NULL,
    passphrase VARCHAR NOT NULL
);

CREATE UNIQUE INDEX ON event_passphrases (event_id, passphrase);
