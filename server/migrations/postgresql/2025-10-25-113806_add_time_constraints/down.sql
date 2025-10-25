ALTER TABLE events
    DROP CONSTRAINT events_date_range;
ALTER TABLE entries
    DROP CONSTRAINT entries_time_range;
ALTER TABLE previous_dates
    DROP CONSTRAINT previous_dates_time_range;
ALTER TABLE announcements
    DROP CONSTRAINT announcements_date_range;
