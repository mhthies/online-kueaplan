// @generated automatically by Diesel CLI.

diesel::table! {
    announcement_categories (announcement_id, category_id) {
        announcement_id -> Uuid,
        category_id -> Uuid,
    }
}

diesel::table! {
    announcement_rooms (announcement_id, room_id) {
        announcement_id -> Uuid,
        room_id -> Uuid,
    }
}

diesel::table! {
    announcements (id) {
        id -> Uuid,
        event_id -> Int4,
        announcement_type -> Int4,
        text -> Varchar,
        show_with_days -> Bool,
        begin_date -> Nullable<Date>,
        end_date -> Nullable<Date>,
        show_with_categories -> Bool,
        show_with_all_categories -> Bool,
        show_with_rooms -> Bool,
        show_with_all_rooms -> Bool,
        sort_key -> Int4,
        deleted -> Bool,
        last_updated -> Timestamptz,
    }
}

diesel::table! {
    categories (id) {
        id -> Uuid,
        title -> Varchar,
        icon -> Varchar,
        #[max_length = 6]
        color -> Bpchar,
        event_id -> Int4,
        deleted -> Bool,
        last_updated -> Timestamptz,
        is_official -> Bool,
        sort_key -> Int4,
    }
}

diesel::table! {
    entries (id) {
        id -> Uuid,
        title -> Varchar,
        description -> Varchar,
        responsible_person -> Varchar,
        is_room_reservation -> Bool,
        event_id -> Int4,
        begin -> Timestamptz,
        end -> Timestamptz,
        category -> Uuid,
        deleted -> Bool,
        last_updated -> Timestamptz,
        comment -> Varchar,
        time_comment -> Varchar,
        room_comment -> Varchar,
        is_exclusive -> Bool,
        is_cancelled -> Bool,
    }
}

diesel::table! {
    entry_rooms (entry_id, room_id) {
        entry_id -> Uuid,
        room_id -> Uuid,
    }
}

diesel::table! {
    event_passphrases (id) {
        id -> Int4,
        event_id -> Int4,
        privilege -> Int4,
        passphrase -> Nullable<Varchar>,
        derivable_from_passphrase -> Nullable<Int4>,
    }
}

diesel::table! {
    events (id) {
        id -> Int4,
        title -> Varchar,
        begin_date -> Date,
        end_date -> Date,
    }
}

diesel::table! {
    previous_date_rooms (previous_date_id, room_id) {
        previous_date_id -> Uuid,
        room_id -> Uuid,
    }
}

diesel::table! {
    previous_dates (id) {
        id -> Uuid,
        entry_id -> Uuid,
        comment -> Varchar,
        begin -> Timestamptz,
        end -> Timestamptz,
        last_updated -> Timestamptz,
    }
}

diesel::table! {
    rooms (id) {
        id -> Uuid,
        title -> Varchar,
        description -> Varchar,
        event_id -> Int4,
        deleted -> Bool,
        last_updated -> Timestamptz,
    }
}

diesel::joinable!(announcement_categories -> announcements (announcement_id));
diesel::joinable!(announcement_categories -> categories (category_id));
diesel::joinable!(announcement_rooms -> announcements (announcement_id));
diesel::joinable!(announcement_rooms -> rooms (room_id));
diesel::joinable!(announcements -> events (event_id));
diesel::joinable!(categories -> events (event_id));
diesel::joinable!(entries -> categories (category));
diesel::joinable!(entries -> events (event_id));
diesel::joinable!(entry_rooms -> entries (entry_id));
diesel::joinable!(entry_rooms -> rooms (room_id));
diesel::joinable!(event_passphrases -> events (event_id));
diesel::joinable!(previous_date_rooms -> previous_dates (previous_date_id));
diesel::joinable!(previous_date_rooms -> rooms (room_id));
diesel::joinable!(previous_dates -> entries (entry_id));
diesel::joinable!(rooms -> events (event_id));

diesel::allow_tables_to_appear_in_same_query!(
    announcement_categories,
    announcement_rooms,
    announcements,
    categories,
    entries,
    entry_rooms,
    event_passphrases,
    events,
    previous_date_rooms,
    previous_dates,
    rooms,
);
