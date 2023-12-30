use chrono::{naive::NaiveDate, DateTime};
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Queryable)]
pub struct Event {
    pub id: i32,
    pub title: String,
    pub begin_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Queryable, Insertable, AsChangeset, Identifiable)]
#[diesel(table_name=super::schema::entries)]
pub struct Entry {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub responsible_person: String,
    pub is_blocker: bool,
    pub residue_of: Option<Uuid>,
    pub event_id: i32,
}

#[derive(Queryable)]
pub struct Room {
    pub id: Uuid,
    pub title: String,
    pub description: String,
}

pub struct FullEntry {
    pub entry: Entry,
    pub room_ids: Vec<Uuid>,
}

// Introduce type for Entry-Room-association, to simplify grouped retrieval of room_ids of an Entry
// using Diesel's .grouped_by() method.
#[derive(Queryable, Associations, Identifiable)]
#[diesel(table_name=super::schema::entry_rooms)]
#[diesel(primary_key(entry_id, room_id))]
#[diesel(belongs_to(Entry))]
pub struct EntryRoomMapping {
    pub entry_id: Uuid,
    pub room_id: Uuid,
}

impl FullEntry {
    pub fn from_api(entry: kueaplan_api_types::Entry, event_id: i32) -> Self {
        Self {
            entry: Entry {
                id: entry.id,
                title: entry.title,
                description: entry.description,
                responsible_person: entry.responsible_person,
                is_blocker: entry.is_blocker,
                residue_of: entry.residue_of,
                event_id,
            },
            room_ids: entry.room,
        }
    }

    pub fn into_api(self) -> kueaplan_api_types::Entry {
        kueaplan_api_types::Entry {
            id: self.entry.id,
            title: self.entry.title,
            description: self.entry.description,
            room: self.room_ids,
            begin: DateTime::from_timestamp(0, 0).unwrap(),  // TODO
            end: DateTime::from_timestamp(0, 0).unwrap(),  // TODO
            responsible_person: self.entry.responsible_person,
            is_blocker: self.entry.is_blocker,
            residue_of: self.entry.residue_of,
            category: None,  // TODO
        }
    }
}
