// @generated automatically by Diesel CLI.

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
    }
}

diesel::table! {
    entries (id) {
        id -> Uuid,
        title -> Varchar,
        description -> Varchar,
        responsible_person -> Varchar,
        is_blocker -> Bool,
        residue_of -> Nullable<Uuid>,
        event_id -> Int4,
        begin -> Timestamptz,
        end -> Timestamptz,
        category -> Nullable<Uuid>,
        deleted -> Bool,
        last_updated -> Timestamptz,
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
        passphrase -> Varchar,
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
    rooms (id) {
        id -> Uuid,
        title -> Varchar,
        description -> Varchar,
        event_id -> Int4,
        deleted -> Bool,
        last_updated -> Timestamptz,
    }
}

diesel::joinable!(categories -> events (event_id));
diesel::joinable!(entries -> categories (category));
diesel::joinable!(entries -> events (event_id));
diesel::joinable!(entry_rooms -> entries (entry_id));
diesel::joinable!(entry_rooms -> rooms (room_id));
diesel::joinable!(event_passphrases -> events (event_id));
diesel::joinable!(rooms -> events (event_id));

diesel::allow_tables_to_appear_in_same_query!(
    categories,
    entries,
    entry_rooms,
    event_passphrases,
    events,
    rooms,
);
