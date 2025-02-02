
use uuid::Uuid;
use chrono::{naive::NaiveDate, DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Event {
    pub id: i32,
    pub title: String,
    pub begin_date: NaiveDate,
    pub end_date: NaiveDate,
}

fn not(v: &bool) -> bool {!v}

#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub id: Uuid,
    pub title: String,
    #[serde(default,skip_serializing_if="str::is_empty")]
    pub comment: String,
    #[serde(default,skip_serializing_if="str::is_empty")]
    pub description: String,
    pub room: Vec<Uuid>,
    #[serde(default,skip_serializing_if="str::is_empty",rename = "roomComment")]
    pub room_comment: String,
    pub begin: DateTime<Utc>,
    pub end: DateTime<Utc>,
    #[serde(default,skip_serializing_if="str::is_empty",rename = "timeComment")]
    pub time_comment: String,
    #[serde(default,rename="responsiblePerson")]
    pub responsible_person: String,
    #[serde(default,skip_serializing_if="not",rename="isExclusive")]
    pub is_room_reservation: bool,
    #[serde(default,skip_serializing_if="not",rename="isCancelled")]
    pub is_exclusive: bool,
    #[serde(default,skip_serializing_if="not",rename="isRoomReservation")]
    pub is_cancelled: bool,
    #[serde(default,skip_serializing_if="Option::is_none",rename="residueOf")]
    pub residue_of: Option<Uuid>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub category: Option<Uuid>,
}

#[derive(Serialize, Deserialize)]
pub struct Room {
    pub id: Uuid,
    pub title: String,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub struct Category {
    pub id: Uuid,
    pub title: String,
    pub icon: String,
    pub color: String,
    #[serde(default,skip_serializing_if="not",rename="isOfficial")]
    pub is_official: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Updates {
    #[serde(rename="changedEntries")]
    pub changed_entries: Vec<Entry>,
    #[serde(rename="deletedEntries")]
    pub deleted_entries: Vec<uuid::Uuid>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub rooms: Option<Vec<Room>>,
}
