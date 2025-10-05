use chrono::{naive::NaiveDate, DateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Event {
    pub id: i32,
    pub title: String,
    #[serde(rename = "beginDate")]
    pub begin_date: NaiveDate,
    #[serde(rename = "endDate")]
    pub end_date: NaiveDate,
}

#[derive(Serialize, Deserialize)]
pub struct ExtendedEvent {
    #[serde(flatten)]
    pub basic_data: Event,
    pub timezone: String,
    #[serde(rename = "effectiveBeginOfDay")]
    pub effective_begin_of_day: NaiveTime,
    #[serde(rename = "defaultTimeSchedule")]
    pub default_time_schedule: EventDayTimeSchedule,
}

#[derive(Serialize, Deserialize)]
pub struct EventDayTimeSchedule {
    pub sections: Vec<EventDayScheduleSection>,
}

#[derive(Serialize, Deserialize)]
pub struct EventDayScheduleSection {
    pub name: String,
    #[serde(rename = "endTime")]
    pub end_time: Option<NaiveTime>,
}

fn not(v: &bool) -> bool {
    !v
}

#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub id: Uuid,
    pub title: String,
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub comment: String,
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub description: String,
    pub room: Vec<Uuid>,
    #[serde(default, skip_serializing_if = "str::is_empty", rename = "roomComment")]
    pub room_comment: String,
    pub begin: DateTime<Utc>,
    pub end: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "str::is_empty", rename = "timeComment")]
    pub time_comment: String,
    #[serde(default, rename = "responsiblePerson")]
    pub responsible_person: String,
    #[serde(default, skip_serializing_if = "not", rename = "isExclusive")]
    pub is_exclusive: bool,
    #[serde(default, skip_serializing_if = "not", rename = "isCancelled")]
    pub is_cancelled: bool,
    #[serde(default, skip_serializing_if = "not", rename = "isRoomReservation")]
    pub is_room_reservation: bool,
    pub category: Uuid,
    #[serde(default, rename = "previousDates")]
    pub previous_dates: Vec<PreviousDate>,
}

#[derive(Serialize, Deserialize)]
pub struct PreviousDate {
    pub id: Uuid,
    pub begin: DateTime<Utc>,
    pub end: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub comment: String,
    pub room: Vec<Uuid>,
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
    #[serde(default, skip_serializing_if = "not", rename = "isOfficial")]
    pub is_official: bool,
    pub sort_key: i32,
}

#[derive(Serialize, Deserialize)]
pub enum AnnouncementType {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warning")]
    Warning,
}

#[derive(Serialize, Deserialize)]
pub struct Announcement {
    pub id: Uuid,
    #[serde(rename = "announcementType")]
    pub announcement_type: AnnouncementType,
    pub text: String,
    #[serde(default)]
    #[serde(rename = "showWithDays")]
    pub show_with_days: bool,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "beginDate")]
    pub begin_date: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "endDate")]
    pub end_date: Option<NaiveDate>,
    #[serde(rename = "sortKey")]
    pub sort_key: i32,
    #[serde(default, rename = "showWithCategories")]
    pub show_with_categories: bool,
    #[serde(default)]
    pub categories: Vec<Uuid>,
    #[serde(default, rename = "showWithAllCategories")]
    pub show_with_all_categories: bool,
    #[serde(default, rename = "showWithRooms")]
    pub show_with_rooms: bool,
    #[serde(default)]
    pub rooms: Vec<Uuid>,
    #[serde(default, rename = "showWithAllRooms")]
    pub show_with_all_rooms: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Updates {
    #[serde(rename = "changedEntries")]
    pub changed_entries: Vec<Entry>,
    #[serde(rename = "deletedEntries")]
    pub deleted_entries: Vec<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rooms: Option<Vec<Room>>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum AuthorizationRole {
    #[serde(rename = "participant")]
    Participant,
    #[serde(rename = "orga")]
    Orga,
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "participant-sharable")]
    ParticipantSharable,
}

#[derive(Serialize, Deserialize)]
pub struct Authorization {
    pub role: AuthorizationRole,
}

#[derive(Serialize, Deserialize)]
pub struct AuthorizationInfo {
    pub authorization: Vec<Authorization>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Passphrase {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
    #[serde(
        default,
        rename = "derivableFromPassphrase",
        skip_serializing_if = "Option::is_none"
    )]
    pub derivable_from_passphrase: Option<i32>,
    pub role: AuthorizationRole,
}
