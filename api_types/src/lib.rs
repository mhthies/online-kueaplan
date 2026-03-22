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
    pub slug: Option<String>,
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
    #[serde(rename = "precedingEventId")]
    pub preceding_event_id: Option<i32>,
    #[serde(rename = "subsequentEventId")]
    pub subsequent_event_id: Option<i32>,
    #[serde(rename = "entrySubmissionMode")]
    pub entry_submission_mode: EntrySubmissionMode,
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

#[derive(Serialize, Deserialize)]
pub enum EntrySubmissionMode {
    /// No submission of entries by participants
    #[serde(rename = "disabled")]
    Disabled = 0,
    /// Entries can be submitted by participants, but only in state SubmittedForReview, such that
    /// they have to be reviewd by event orgas before being visible to all participants.
    #[serde(rename = "review-before-publishing")]
    ReviewBeforePublishing = 1,
    /// Entries can be submitted by participants, in state PreliminaryPublished, such that they will
    /// be directly visible to all participants, but are marked for later review be orgas.
    #[serde(rename = "review-after-publishing")]
    ReviewAfterPublishing = 2,
}

/// Simple helper function to be used with `#[serde(skip_serializing_if=...)]` for serializing
/// optional bool values.
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
    #[serde(default = "EntryState::default_from_api")]
    pub state: EntryState,
    #[serde(default, rename = "previousDates")]
    pub previous_dates: Vec<PreviousDate>,
}

#[derive(Serialize, Deserialize)]
pub struct EntryPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room: Option<Vec<Uuid>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "roomComment"
    )]
    pub room_comment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub begin: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<DateTime<Utc>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "timeComment"
    )]
    pub time_comment: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "responsiblePerson"
    )]
    pub responsible_person: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "isExclusive"
    )]
    pub is_exclusive: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "isCancelled"
    )]
    pub is_cancelled: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "isRoomReservation"
    )]
    pub is_room_reservation: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<EntryState>,
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
pub enum EntryState {
    /// Normal public entry state, visible to all participants.
    #[serde(rename = "published")]
    Published = 0,
    /// Entry has been created by an orga, but marked as a draft to be not published (visible to
    /// participants) yet
    #[serde(rename = "draft")]
    Draft = 1,
    /// Entry has been submitted by a participant and needs to be reviewed by an orga before
    /// publishing (making it visible to participants)
    #[serde(rename = "submitted-for-review")]
    SubmittedForReview = 2,
    /// Entry is published but still awaiting review by an orga
    #[serde(rename = "preliminary-published")]
    PreliminaryPublished = 3,
    /// Entry has been retracted, so it's currently not visible to participants (but can be
    /// published again later)
    #[serde(rename = "retracted")]
    Retracted = 4,
    /// Entry was submitted by a participant and has been rejected from publishing in review
    #[serde(rename = "rejected")]
    Rejected = 5,
}

impl EntryState {
    fn default_from_api() -> Self {
        Self::Published
    }
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
pub struct AnnouncementPatch {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "announcementType"
    )]
    pub announcement_type: Option<AnnouncementType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "showWithDays"
    )]
    pub show_with_days: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "beginDate")]
    pub begin_date: Option<Option<NaiveDate>>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "endDate")]
    pub end_date: Option<Option<NaiveDate>>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "sortKey")]
    pub sort_key: Option<i32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "showWithCategories"
    )]
    pub show_with_categories: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<Uuid>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "showWithAllCategories"
    )]
    pub show_with_all_categories: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "showWithRooms"
    )]
    pub show_with_rooms: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rooms: Option<Vec<Uuid>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "showWithAllRooms"
    )]
    pub show_with_all_rooms: Option<bool>,
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
    #[serde(rename = "eventId")]
    pub event_id: i32,
    pub authorization: Vec<Authorization>,
}

#[derive(Serialize, Deserialize)]
pub struct AllEventsAuthorizationInfo {
    pub events: Vec<AuthorizationInfo>,
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
    #[serde(default)]
    pub comment: String,
    #[serde(default, rename = "validFrom")]
    pub valid_from: Option<DateTime<Utc>>,
    #[serde(default, rename = "validUntil")]
    pub valid_until: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PassphrasePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "validFrom")]
    pub valid_from: Option<Option<DateTime<Utc>>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "validUntil"
    )]
    pub valid_until: Option<Option<DateTime<Utc>>>,
}
