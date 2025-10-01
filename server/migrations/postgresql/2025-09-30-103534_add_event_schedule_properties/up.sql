
ALTER TABLE events
    ADD timezone VARCHAR NOT NULL DEFAULT 'Europe/Berlin',
    ADD effective_begin_of_day TIME NOT NULL DEFAULT '05:30:00',
    ADD default_time_schedule JSONB NOT NULL DEFAULT $$
    {
        "sections": [
            {"name": "vom Vortag", "end_time": "05:30:00"},
            {"name": "Morgens", "end_time": "12:00:00"},
            {"name": "Mittags", "end_time": "18:00:00"},
            {"name": "Abends", "end_time": null}
        ]
    }
    $$;

ALTER TABLE events
    ALTER timezone DROP DEFAULT,
    ALTER effective_begin_of_day DROP DEFAULT,
    ALTER default_time_schedule DROP DEFAULT;
