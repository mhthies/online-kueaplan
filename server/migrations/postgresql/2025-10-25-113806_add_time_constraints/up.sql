ALTER TABLE events
    ADD CONSTRAINT events_date_range CHECK (end_date >= begin_date);
ALTER TABLE entries
    ADD CONSTRAINT entries_time_range CHECK ("end" >= "begin");
ALTER TABLE previous_dates
    ADD CONSTRAINT previous_dates_time_range CHECK ("end" >= "begin");
ALTER TABLE announcements
    ADD CONSTRAINT announcements_date_range CHECK (begin_date IS NULL OR end_date IS NULL OR end_date >= begin_date);
