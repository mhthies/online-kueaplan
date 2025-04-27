use crate::data_store::EntryId;
use chrono::{naive::NaiveDate, DateTime, Utc};
use diesel::associations::BelongsTo;
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Clone, Queryable)]
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

#[derive(Clone, Queryable, Identifiable, Selectable)]
#[diesel(table_name=super::schema::entries)]
pub struct Entry {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub responsible_person: String,
    pub is_room_reservation: bool,
    pub event_id: i32,
    pub begin: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub category: Uuid,
    pub last_updated: DateTime<Utc>,
    pub comment: String,
    pub time_comment: String,
    pub room_comment: String,
    pub is_exclusive: bool,
    pub is_cancelled: bool,
}

#[derive(Clone)]
pub struct FullEntry {
    pub entry: Entry,
    pub room_ids: Vec<Uuid>,
    pub previous_dates: Vec<FullPreviousDate>,
}

#[derive(Clone, Insertable, AsChangeset, Identifiable)]
#[diesel(table_name=super::schema::entries)]
pub struct NewEntry {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub responsible_person: String,
    pub is_room_reservation: bool,
    pub event_id: i32,
    pub begin: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub category: Uuid,
    pub comment: String,
    pub time_comment: String,
    pub room_comment: String,
    pub is_exclusive: bool,
    pub is_cancelled: bool,
}

#[derive(Clone)]
pub struct FullNewEntry {
    pub entry: NewEntry,
    pub room_ids: Vec<Uuid>,
    pub previous_dates: Vec<FullPreviousDate>,
}

#[derive(Clone, Queryable, Identifiable, Selectable)]
#[diesel(table_name=super::schema::rooms)]
pub struct Room {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub event_id: i32,
    pub last_updated: DateTime<Utc>,
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
#[derive(Queryable, Associations, Identifiable, Selectable)]
#[diesel(table_name=super::schema::entry_rooms)]
#[diesel(primary_key(entry_id, room_id))]
#[diesel(belongs_to(Entry))]
pub struct EntryRoomMapping {
    pub entry_id: Uuid,
    pub room_id: Uuid,
}
#[derive(Queryable, Associations, Identifiable, Selectable)]
#[diesel(table_name=super::schema::previous_date_rooms)]
#[diesel(primary_key(previous_date_id, room_id))]
#[diesel(belongs_to(PreviousDate))]
pub struct PreviousDateRoomMapping {
    pub previous_date_id: Uuid,
    pub room_id: Uuid,
}

#[derive(Clone, Queryable, Selectable, Associations, Insertable, AsChangeset, Identifiable)]
#[diesel(table_name=super::schema::previous_dates)]
#[diesel(belongs_to(Entry))]
pub struct PreviousDate {
    pub id: Uuid,
    pub entry_id: Uuid,
    pub comment: String,
    pub begin: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Clone)]
pub struct FullPreviousDate {
    pub previous_date: PreviousDate,
    pub room_ids: Vec<Uuid>,
}

impl BelongsTo<Entry> for FullPreviousDate {
    type ForeignKey = Uuid;
    type ForeignKeyColumn = super::schema::previous_dates::columns::entry_id;

    fn foreign_key(&self) -> Option<&Self::ForeignKey> {
        Some(&self.previous_date.entry_id)
    }

    fn foreign_key_column() -> Self::ForeignKeyColumn {
        super::schema::previous_dates::columns::entry_id
    }
}

#[derive(Clone, Queryable, Identifiable, Selectable)]
#[diesel(table_name=super::schema::categories)]
pub struct Category {
    pub id: Uuid,
    pub title: String,
    pub icon: String,
    pub color: String,
    pub event_id: i32,
    pub is_official: bool,
    pub last_updated: DateTime<Utc>,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name=super::schema::categories)]
pub struct NewCategory {
    pub id: Uuid,
    pub title: String,
    pub icon: String,
    pub color: String,
    pub event_id: i32,
    pub is_official: bool,
}

impl From<kueaplan_api_types::Event> for NewEvent {
    fn from(value: kueaplan_api_types::Event) -> Self {
        Self {
            title: value.title,
            begin_date: value.begin_date,
            end_date: value.end_date,
        }
    }
}

impl From<Event> for kueaplan_api_types::Event {
    fn from(value: Event) -> Self {
        Self {
            id: value.id,
            title: value.title,
            begin_date: value.begin_date,
            end_date: value.end_date,
        }
    }
}

impl FullNewEntry {
    pub fn from_api(entry: kueaplan_api_types::Entry, event_id: i32) -> Self {
        Self {
            entry: NewEntry {
                id: entry.id,
                title: entry.title,
                description: entry.description,
                responsible_person: entry.responsible_person,
                is_room_reservation: entry.is_room_reservation,
                event_id,
                begin: entry.begin,
                end: entry.end,
                category: entry.category,
                comment: entry.comment,
                room_comment: entry.room_comment,
                time_comment: entry.time_comment,
                is_exclusive: entry.is_exclusive,
                is_cancelled: entry.is_cancelled,
            },
            room_ids: entry.room,
            previous_dates: entry
                .previous_dates
                .into_iter()
                .map(|pd| FullPreviousDate::from_api(pd, entry.id))
                .collect(),
        }
    }
}

impl From<FullEntry> for FullNewEntry {
    fn from(value: FullEntry) -> Self {
        FullNewEntry {
            entry: NewEntry {
                id: value.entry.id,
                title: value.entry.title,
                description: value.entry.description,
                responsible_person: value.entry.responsible_person,
                is_room_reservation: value.entry.is_room_reservation,
                event_id: value.entry.event_id,
                begin: value.entry.begin,
                end: value.entry.end,
                category: value.entry.category,
                comment: value.entry.comment,
                time_comment: value.entry.time_comment,
                room_comment: value.entry.room_comment,
                is_exclusive: value.entry.is_exclusive,
                is_cancelled: value.entry.is_cancelled,
            },
            room_ids: value.room_ids,
            previous_dates: value.previous_dates,
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
            is_room_reservation: value.entry.is_room_reservation,
            category: value.entry.category,
            comment: value.entry.comment,
            room_comment: value.entry.room_comment,
            time_comment: value.entry.time_comment,
            is_exclusive: value.entry.is_exclusive,
            is_cancelled: value.entry.is_cancelled,
            previous_dates: value
                .previous_dates
                .into_iter()
                .map(|pd| pd.into())
                .collect(),
        }
    }
}

impl From<FullPreviousDate> for kueaplan_api_types::PreviousDate {
    fn from(value: FullPreviousDate) -> Self {
        Self {
            id: value.previous_date.id,
            begin: value.previous_date.begin,
            end: value.previous_date.end,
            comment: value.previous_date.comment,
            room: value.room_ids,
        }
    }
}

impl FullPreviousDate {
    fn from_api(value: kueaplan_api_types::PreviousDate, entry_id: EntryId) -> Self {
        Self {
            previous_date: PreviousDate {
                id: value.id,
                entry_id,
                comment: value.comment,
                begin: value.begin,
                end: value.end,
            },
            room_ids: value.room,
        }
    }
}

impl NewRoom {
    pub fn from_api(room: kueaplan_api_types::Room, event_id: i32) -> Self {
        Self {
            id: room.id,
            title: room.title,
            description: room.description,
            event_id,
        }
    }
}

impl From<Room> for kueaplan_api_types::Room {
    fn from(value: Room) -> Self {
        kueaplan_api_types::Room {
            id: value.id,
            title: value.title,
            description: value.description,
        }
    }
}

impl NewCategory {
    pub fn from_api(category: kueaplan_api_types::Category, event_id: i32) -> Self {
        Self {
            id: category.id,
            title: category.title,
            icon: category.icon,
            color: category.color,
            event_id,
            is_official: category.is_official,
        }
    }
}

impl From<Category> for kueaplan_api_types::Category {
    fn from(value: Category) -> Self {
        kueaplan_api_types::Category {
            id: value.id,
            title: value.title,
            icon: value.icon,
            color: value.color,
            is_official: value.is_official,
        }
    }
}
