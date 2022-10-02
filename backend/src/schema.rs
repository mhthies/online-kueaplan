// @generated automatically by Diesel CLI.

diesel::table! {
    entries (id) {
        id -> Uuid,
        title -> Varchar,
        description -> Varchar,
        responsible_person -> Varchar,
        is_blocker -> Bool,
        residue_of -> Nullable<Uuid>,
        event_id -> Int4,
    }
}

diesel::table! {
    entry_rooms (entry_id, room_id) {
        entry_id -> Uuid,
        room_id -> Uuid,
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
    }
}

diesel::joinable!(entries -> events (event_id));
diesel::joinable!(entry_rooms -> entries (entry_id));
diesel::joinable!(entry_rooms -> rooms (room_id));
diesel::joinable!(rooms -> events (event_id));

diesel::allow_tables_to_appear_in_same_query!(
    entries,
    entry_rooms,
    events,
    rooms,
);
