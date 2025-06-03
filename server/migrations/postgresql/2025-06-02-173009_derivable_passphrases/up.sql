ALTER TABLE event_passphrases
    ALTER COLUMN passphrase DROP NOT NULL,
    ADD COLUMN  derivable_from_passphrase INTEGER REFERENCES event_passphrases(id);

comment on COLUMN event_passphrases.passphrase is 'if NULL, this passphrase can only derived from another one';
