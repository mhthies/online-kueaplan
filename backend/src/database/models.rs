use chrono::{naive::NaiveDate, DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Queryable)]
pub struct Event {
    pub id: i32,
    pub title: String,
    pub begin_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Insertable)]
#[diesel(table_name=super::schema::events)]
pub struct NewEvent {
    pub title: String,
    pub begin_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Queryable, Identifiable)]
#[diesel(table_name=super::schema::entries)]
pub struct Entry {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub responsible_person: String,
    pub is_blocker: bool,
    pub residue_of: Option<Uuid>,
    pub event_id: i32,
    pub begin: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub category: Option<Uuid>,
    pub deleted: bool,
    pub last_updated: DateTime<Utc>,
}

pub struct FullEntry {
    pub entry: Entry,
    pub room_ids: Vec<Uuid>,
}

#[derive(Insertable, AsChangeset, Identifiable)]
#[diesel(table_name=super::schema::entries)]
pub struct NewEntry {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub responsible_person: String,
    pub is_blocker: bool,
    pub residue_of: Option<Uuid>,
    pub event_id: i32,
    pub begin: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub category: Option<Uuid>,
}

pub struct FullNewEntry {
    pub entry: NewEntry,
    pub room_ids: Vec<Uuid>,
}

#[derive(Queryable, Identifiable, Selectable)]
#[diesel(table_name=super::schema::rooms)]
pub struct Room {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub event_id: i32,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name=super::schema::rooms)]
pub struct NewRoom {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub event_id: i32,
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

#[derive(Queryable, Identifiable, Selectable)]
#[diesel(table_name=super::schema::categories)]
pub struct Category {
    pub id: Uuid,
    pub title: String,
    pub icon: String,
    pub color: String,
    pub event_id: i32,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name=super::schema::categories)]
pub struct NewCategory {
    pub id: Uuid,
    pub title: String,
    pub icon: String,
    pub color: String,
    pub event_id: i32,
}

impl FullNewEntry {
    pub fn from_api(entry: kueaplan_api_types::Entry, event_id: i32) -> Self {
        Self {
            entry: NewEntry {
                id: entry.id,
                title: entry.title,
                description: entry.description,
                responsible_person: entry.responsible_person,
                is_blocker: entry.is_blocker,
                residue_of: entry.residue_of,
                event_id,
                begin: entry.begin,
                end: entry.end,
                category: entry.category,
            },
            room_ids: entry.room,
        }
    }
}

impl From<FullEntry> for kueaplan_api_types::Entry {
    fn from(value: FullEntry) -> Self {
        kueaplan_api_types::Entry {
            id: value.entry.id,
            title: value.entry.title,
            description: value.entry.description,
            room: value.room_ids,
            begin: value.entry.begin,
            end: value.entry.end,
            responsible_person: value.entry.responsible_person,
            is_blocker: value.entry.is_blocker,
            residue_of: value.entry.residue_of,
            category: value.entry.category,
        }
    }
}
