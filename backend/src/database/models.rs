use chrono::naive::NaiveDate;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
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
#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub id: Uuid,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub responsible_person: String,
    #[serde(default)]
    pub is_blocker: bool,
    #[serde(default)]
    pub residue_of: Option<Uuid>,
    #[serde(skip)]
    pub event_id: i32,
}

#[derive(Queryable)]
pub struct Room {
    pub id: Uuid,
    pub title: String,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub struct FullEntry {
    #[serde(flatten)]
    pub entry: Entry,
    #[serde(rename = "room")]
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
