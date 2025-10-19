use crate::data_store::auth_token::AccessRole;
use crate::data_store::{EntryId, EnumMemberNotExistingError, EventId, PassphraseId};
use chrono::{naive::NaiveDate, DateTime, Utc};
use diesel::associations::BelongsTo;
use diesel::deserialize::FromSql;
use diesel::prelude::*;
use diesel::query_builder::bind_collector::RawBytesBindCollector;
use diesel::serialize::ToSql;
use diesel::{AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name=super::schema::events)]
pub struct Event {
    pub id: i32,
    pub title: String,
    pub begin_date: NaiveDate,
    pub end_date: NaiveDate,
    pub slug: Option<String>,
}

impl From<kueaplan_api_types::Event> for Event {
    fn from(value: kueaplan_api_types::Event) -> Self {
        Self {
            id: value.id,
            title: value.title,
            begin_date: value.begin_date,
            end_date: value.end_date,
            slug: value.slug,
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
            slug: value.slug,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name=super::schema::events)]
pub struct NewEvent {
    pub title: String,
    pub begin_date: NaiveDate,
    pub end_date: NaiveDate,
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

#[derive(Clone, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name=super::schema::events)]
pub struct ExtendedEvent {
    #[diesel(embed)]
    pub basic_data: Event,
    #[diesel(embed)]
    pub clock_info: EventClockInfo,
    pub default_time_schedule: EventDayTimeSchedule,
    pub preceding_event_id: Option<EventId>,
    pub subsequent_event_id: Option<EventId>,
}

impl TryFrom<kueaplan_api_types::ExtendedEvent> for ExtendedEvent {
    type Error = String;

    fn try_from(value: kueaplan_api_types::ExtendedEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            basic_data: value.basic_data.into(),
            clock_info: EventClockInfo {
                timezone: value.timezone.parse().map_err(|e| format!("{:?}", e))?,
                effective_begin_of_day: value.effective_begin_of_day,
            },
            default_time_schedule: value.default_time_schedule.into(),
            preceding_event_id: value.preceding_event_id,
            subsequent_event_id: value.subsequent_event_id,
        })
    }
}

impl From<ExtendedEvent> for kueaplan_api_types::ExtendedEvent {
    fn from(value: ExtendedEvent) -> Self {
        Self {
            basic_data: value.basic_data.into(),
            timezone: value.clock_info.timezone.to_string(),
            effective_begin_of_day: value.clock_info.effective_begin_of_day,
            default_time_schedule: value.default_time_schedule.into(),
            preceding_event_id: value.preceding_event_id,
            subsequent_event_id: value.subsequent_event_id,
        }
    }
}

#[derive(Clone, Debug, Queryable, Selectable, AsChangeset, Insertable)]
#[diesel(table_name=super::schema::events)]
pub struct EventClockInfo {
    #[diesel(serialize_as=super::util::TimezoneWrapper, deserialize_as=super::util::TimezoneWrapper)]
    pub timezone: chrono_tz::Tz,
    pub effective_begin_of_day: chrono::NaiveTime,
}

// Manual implementation of diesel::insertable::Insertable for &EventClockInfo, because the derive
// macro only creates an implementation for EventClockInfo (not the reference) when
// `#[diesel(serialize_as=...)` is used on a field. The trait implementation for the reference type
// is in turn required for using the type with `#[diesel(embed)]` and deriving `Insertable` on the
// outer type.
// This manual implementation is also kind of a hack, because it actually uses the owned
// TimezoneWrapper in the resulting SQL expression/values struct, where the derived trait
// implementation would normally use a reference.
impl<'insert> Insertable<super::schema::events::table> for &'insert EventClockInfo {
    type Values = <(
        Option<diesel::dsl::Eq<super::schema::events::timezone, super::util::TimezoneWrapper>>,
        Option<
            diesel::dsl::Eq<
                super::schema::events::effective_begin_of_day,
                &'insert chrono::NaiveTime,
            >,
        >,
    ) as Insertable<super::schema::events::table>>::Values;

    fn values(self) -> Self::Values {
        Insertable::<super::schema::events::table>::values((
            Some(diesel::ExpressionMethods::eq(
                super::schema::events::timezone,
                super::util::TimezoneWrapper::from(self.timezone),
            )),
            Some(diesel::ExpressionMethods::eq(
                super::schema::events::effective_begin_of_day,
                &self.effective_begin_of_day,
            )),
        ))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct EventDayTimeSchedule {
    pub sections: Vec<EventDayScheduleSection>,
}

impl<DB> FromSql<diesel::sql_types::Jsonb, DB> for EventDayTimeSchedule
where
    DB: diesel::backend::Backend,
    serde_json::Value: FromSql<diesel::sql_types::Jsonb, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let value = serde_json::Value::from_sql(bytes)?;
        Ok(serde_json::from_value(value)?)
    }
}

impl<DB> ToSql<diesel::sql_types::Jsonb, DB> for EventDayTimeSchedule
where
    DB: diesel::backend::Backend,
    for<'c> DB: diesel::backend::Backend<BindCollector<'c> = RawBytesBindCollector<DB>>,
    serde_json::Value: ToSql<diesel::sql_types::Jsonb, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        let value = serde_json::to_value(self)?;
        value.to_sql(&mut out.reborrow())
    }
}

impl From<kueaplan_api_types::EventDayTimeSchedule> for EventDayTimeSchedule {
    fn from(value: kueaplan_api_types::EventDayTimeSchedule) -> Self {
        Self {
            sections: value.sections.into_iter().map(|s| s.into()).collect(),
        }
    }
}

impl From<EventDayTimeSchedule> for kueaplan_api_types::EventDayTimeSchedule {
    fn from(value: EventDayTimeSchedule) -> Self {
        Self {
            sections: value.sections.into_iter().map(|s| s.into()).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EventDayScheduleSection {
    pub name: String,
    pub end_time: Option<chrono::NaiveTime>,
}

impl From<kueaplan_api_types::EventDayScheduleSection> for EventDayScheduleSection {
    fn from(value: kueaplan_api_types::EventDayScheduleSection) -> Self {
        Self {
            name: value.name,
            end_time: value.end_time,
        }
    }
}

impl From<EventDayScheduleSection> for kueaplan_api_types::EventDayScheduleSection {
    fn from(value: EventDayScheduleSection) -> Self {
        Self {
            name: value.name,
            end_time: value.end_time,
        }
    }
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

#[derive(Clone, Queryable, Identifiable, Selectable)]
#[diesel(table_name=super::schema::rooms)]
pub struct Room {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub event_id: i32,
    pub last_updated: DateTime<Utc>,
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

#[derive(Insertable, AsChangeset)]
#[diesel(table_name=super::schema::rooms)]
pub struct NewRoom {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub event_id: i32,
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

#[derive(Queryable, Associations, Identifiable, Selectable)]
#[diesel(table_name=super::schema::previous_date_rooms)]
#[diesel(primary_key(previous_date_id, room_id))]
#[diesel(belongs_to(PreviousDate))]
pub struct PreviousDateRoomMapping {
    pub previous_date_id: Uuid,
    pub room_id: Uuid,
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
    pub sort_key: i32,
}

impl From<Category> for kueaplan_api_types::Category {
    fn from(value: Category) -> Self {
        kueaplan_api_types::Category {
            id: value.id,
            title: value.title,
            icon: value.icon,
            color: value.color,
            is_official: value.is_official,
            sort_key: value.sort_key,
        }
    }
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
    pub sort_key: i32,
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
            sort_key: category.sort_key,
        }
    }
}

#[derive(Clone, Queryable, Identifiable, Selectable)]
#[diesel(table_name=super::schema::announcements)]
pub struct Announcement {
    pub id: Uuid,
    pub event_id: i32,
    pub announcement_type: AnnouncementType,
    pub text: String,
    pub show_with_days: bool,
    pub begin_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub show_with_categories: bool,
    pub show_with_all_categories: bool,
    pub show_with_rooms: bool,
    pub show_with_all_rooms: bool,
    pub sort_key: i32,
    pub last_updated: DateTime<Utc>,
}

#[derive(Clone)]
pub struct FullAnnouncement {
    pub announcement: Announcement,
    pub category_ids: Vec<Uuid>,
    pub room_ids: Vec<Uuid>,
}

impl From<FullAnnouncement> for kueaplan_api_types::Announcement {
    fn from(value: FullAnnouncement) -> Self {
        Self {
            id: value.announcement.id,
            announcement_type: value.announcement.announcement_type.into(),
            text: value.announcement.text,
            show_with_days: value.announcement.show_with_days,
            begin_date: value.announcement.begin_date,
            end_date: value.announcement.end_date,
            sort_key: value.announcement.sort_key,
            show_with_categories: value.announcement.show_with_categories,
            categories: value.category_ids,
            show_with_all_categories: value.announcement.show_with_all_categories,
            show_with_rooms: value.announcement.show_with_rooms,
            rooms: value.room_ids,
            show_with_all_rooms: value.announcement.show_with_all_rooms,
        }
    }
}

#[derive(Clone, Insertable, AsChangeset, Identifiable)]
#[diesel(table_name=super::schema::announcements)]
pub struct NewAnnouncement {
    pub id: Uuid,
    pub event_id: i32,
    pub announcement_type: AnnouncementType,
    pub text: String,
    pub show_with_days: bool,
    pub begin_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub show_with_categories: bool,
    pub show_with_all_categories: bool,
    pub show_with_rooms: bool,
    pub show_with_all_rooms: bool,
    pub sort_key: i32,
}

#[derive(Clone)]
pub struct FullNewAnnouncement {
    pub announcement: NewAnnouncement,
    pub category_ids: Vec<Uuid>,
    pub room_ids: Vec<Uuid>,
}

impl FullNewAnnouncement {
    pub fn from_api(announcement: kueaplan_api_types::Announcement, event_id: EventId) -> Self {
        Self {
            announcement: NewAnnouncement {
                id: announcement.id,
                event_id,
                announcement_type: announcement.announcement_type.into(),
                text: announcement.text,
                show_with_days: announcement.show_with_days,
                begin_date: announcement.begin_date,
                end_date: announcement.end_date,
                show_with_categories: announcement.show_with_categories,
                show_with_all_categories: announcement.show_with_all_categories,
                show_with_rooms: announcement.show_with_rooms,
                show_with_all_rooms: announcement.show_with_all_rooms,
                sort_key: announcement.sort_key,
            },
            category_ids: announcement.categories,
            room_ids: announcement.rooms,
        }
    }
}

impl From<FullAnnouncement> for FullNewAnnouncement {
    fn from(value: FullAnnouncement) -> Self {
        Self {
            announcement: NewAnnouncement {
                id: value.announcement.id,
                event_id: value.announcement.event_id,
                announcement_type: value.announcement.announcement_type,
                text: value.announcement.text,
                show_with_days: value.announcement.show_with_days,
                begin_date: value.announcement.begin_date,
                end_date: value.announcement.end_date,
                show_with_categories: value.announcement.show_with_categories,
                show_with_all_categories: value.announcement.show_with_all_categories,
                show_with_rooms: value.announcement.show_with_rooms,
                show_with_all_rooms: value.announcement.show_with_all_rooms,
                sort_key: value.announcement.sort_key,
            },
            category_ids: value.category_ids,
            room_ids: value.room_ids,
        }
    }
}

#[derive(Debug, PartialEq, FromSqlRow, AsExpression, Eq, Clone, Copy)]
#[diesel(sql_type = diesel::sql_types::Integer)]
#[repr(i32)]
pub enum AnnouncementType {
    Info = 0,
    Warning = 1,
}

impl TryFrom<i32> for AnnouncementType {
    type Error = EnumMemberNotExistingError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AnnouncementType::Info),
            1 => Ok(AnnouncementType::Warning),
            _ => Err(EnumMemberNotExistingError {
                member_value: value,
                enum_name: "AnnouncementType",
            }),
        }
    }
}
impl From<AnnouncementType> for i32 {
    fn from(value: AnnouncementType) -> Self {
        value as i32
    }
}

impl From<AnnouncementType> for kueaplan_api_types::AnnouncementType {
    fn from(value: AnnouncementType) -> Self {
        match value {
            AnnouncementType::Info => Self::Info,
            AnnouncementType::Warning => Self::Warning,
        }
    }
}

impl From<kueaplan_api_types::AnnouncementType> for AnnouncementType {
    fn from(value: kueaplan_api_types::AnnouncementType) -> Self {
        match value {
            kueaplan_api_types::AnnouncementType::Info => Self::Info,
            kueaplan_api_types::AnnouncementType::Warning => Self::Warning,
        }
    }
}

impl<DB> ToSql<diesel::sql_types::Integer, DB> for AnnouncementType
where
    DB: diesel::backend::Backend,
    for<'c> DB: diesel::backend::Backend<BindCollector<'c> = RawBytesBindCollector<DB>>,
    i32: ToSql<diesel::sql_types::Integer, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        let value: i32 = (*self).into();
        value.to_sql(&mut out.reborrow())
    }
}

impl<DB> FromSql<diesel::sql_types::Integer, DB> for AnnouncementType
where
    DB: diesel::backend::Backend,
    i32: FromSql<diesel::sql_types::Integer, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let x = i32::from_sql(bytes)?;
        x.try_into()
            .map_err(|e: EnumMemberNotExistingError| e.to_string().into())
    }
}

// Introduce type for Announcement-Category and Announcement-Room associations, to simplify grouped
// retrieval of category_ids/room_ids of an Announcement, using Diesel's .grouped_by() method.
#[derive(Queryable, Associations, Identifiable, Selectable)]
#[diesel(table_name=super::schema::announcement_categories)]
#[diesel(primary_key(announcement_id, category_id))]
#[diesel(belongs_to(Announcement))]
pub struct AnnouncementCategoryMapping {
    pub announcement_id: Uuid,
    pub category_id: Uuid,
}

#[derive(Queryable, Associations, Identifiable, Selectable)]
#[diesel(table_name=super::schema::announcement_rooms)]
#[diesel(primary_key(announcement_id, room_id))]
#[diesel(belongs_to(Announcement))]
pub struct AnnouncementRoomMapping {
    pub announcement_id: Uuid,
    pub room_id: Uuid,
}

#[derive(Clone, Queryable, Identifiable, Selectable)]
#[diesel(table_name=super::schema::event_passphrases)]
pub struct Passphrase {
    pub id: PassphraseId,
    pub event_id: EventId,
    pub privilege: AccessRole,
    pub passphrase: Option<String>,
    pub derivable_from_passphrase: Option<PassphraseId>,
}

impl From<Passphrase> for kueaplan_api_types::Passphrase {
    fn from(value: Passphrase) -> Self {
        Self {
            id: Some(value.id),
            passphrase: value.passphrase,
            derivable_from_passphrase: value.derivable_from_passphrase,
            role: value.privilege.into(),
        }
    }
}

#[derive(Clone, Insertable)]
#[diesel(table_name=super::schema::event_passphrases)]
pub struct NewPassphrase {
    pub event_id: EventId,
    pub passphrase: Option<String>,
    pub privilege: AccessRole,
    pub derivable_from_passphrase: Option<PassphraseId>,
}

impl NewPassphrase {
    pub fn from_api(passphrase: kueaplan_api_types::Passphrase, event_id: EventId) -> Self {
        Self {
            event_id,
            passphrase: passphrase.passphrase,
            privilege: passphrase.role.into(),
            derivable_from_passphrase: passphrase.derivable_from_passphrase,
        }
    }
}
