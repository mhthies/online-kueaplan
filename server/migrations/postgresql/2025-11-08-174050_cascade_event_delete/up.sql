
ALTER TABLE ONLY announcements
    DROP CONSTRAINT announcements_event_id_fkey,
    ADD CONSTRAINT announcements_event_id_fkey FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE;

ALTER TABLE ONLY categories
    DROP CONSTRAINT categories_event_id_fkey,
    ADD CONSTRAINT categories_event_id_fkey FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE;

ALTER TABLE ONLY entries
    DROP CONSTRAINT entries_event_id_fkey,
    ADD CONSTRAINT entries_event_id_fkey FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE;

ALTER TABLE ONLY rooms
    DROP CONSTRAINT rooms_event_id_fkey,
    ADD CONSTRAINT rooms_event_id_fkey FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE;

ALTER TABLE ONLY event_passphrases
    DROP CONSTRAINT event_passphrases_event_id_fkey,
    ADD CONSTRAINT event_passphrases_event_id_fkey FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE;
