// @generated automatically by Diesel CLI.

diesel::table! {
    entries (id) {
        id -> Uuid,
        title -> Varchar,
        description -> Varchar,
        responsibleperson -> Varchar,
        isblocker -> Bool,
        residueof -> Nullable<Uuid>,
        eventid -> Int4,
    }
}

diesel::table! {
    entry_rooms (entryid, roomid) {
        entryid -> Uuid,
        roomid -> Uuid,
    }
}

diesel::table! {
    events (id) {
        id -> Int4,
        title -> Varchar,
        begindate -> Date,
        enddate -> Date,
    }
}

diesel::table! {
    rooms (id) {
        id -> Uuid,
        title -> Varchar,
        description -> Varchar,
        eventid -> Int4,
    }
}

diesel::joinable!(entries -> events (eventid));
diesel::joinable!(entry_rooms -> entries (entryid));
diesel::joinable!(entry_rooms -> rooms (roomid));
diesel::joinable!(rooms -> events (eventid));

diesel::allow_tables_to_appear_in_same_query!(
    entries,
    entry_rooms,
    events,
    rooms,
);
