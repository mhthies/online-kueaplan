DELETE FROM event_passphrases WHERE passphrase IS NULL;
ALTER TABLE event_passphrases
    DROP COLUMN derivable_from_passphrase,
    ALTER COLUMN passphrase SET NOT NULL;
