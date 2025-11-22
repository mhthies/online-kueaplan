//! The backend part of the backend: the database interface
//!
//! The primary entry point to this module is the function [get_store_from_env], which returns an
//! object implementing the [KueaPlanStore] trait. This object can be shared between threads in a
//! global application state and be used to create [KueaPlanStoreFacade] instances for interaction
//! with the database. These provide a CRUD-like interface, using the data models from the [models]
//! module.
//!
//! The primary implementation of [KueaPlanStore] ([postgres::PgDataStore]) wraps a PostgreSQL
//! connection pool and its corresponding [KueaPlanStoreFacade] objects
//! ([postgres::PgDataStoreFacade]) hold a reference to one pooled connection each, using the Diesel
//! query DSL for implementing the database interaction.
//!
//! Other [KueaPlanStore] implementations may be added later and selected via the "DATABASE_URL"
//! environment variable.

use crate::auth_session::SessionToken;
use crate::cli_error::CliError;
use crate::cli_error::CliError::UnexpectedStoreError;
use crate::data_store::auth_token::Privilege;
use crate::setup;
use auth_token::{AuthToken, GlobalAuthToken};
use std::fmt::{Debug, Display, Formatter};

pub mod auth_token;
pub mod models;
mod postgres;
mod schema;
mod util;

/// Get a [KuaPlanStore] instances, according the "DATABASE_URL" environment variable.
///
/// The DATABASE_URL must be a PosgreSQL connection url, following the schema
/// "postgres://{user}:{password}@{host}/{database}".
pub fn get_store_from_env() -> Result<impl KuaPlanStore, CliError> {
    postgres::PgDataStore::new(&setup::get_database_url_from_env()?)
        .map_err(|err| UnexpectedStoreError(err.to_string()))
}

pub type EventId = i32;
pub type EntryId = uuid::Uuid;
pub type PreviousDateId = uuid::Uuid;
pub type RoomId = uuid::Uuid;
pub type CategoryId = uuid::Uuid;
pub type AnnouncementId = uuid::Uuid;
pub type PassphraseId = i32;

pub trait KueaPlanStoreFacade {
    /// Get a filtered list of events
    ///
    /// Events are returned in chronological order, i.e. sorted by (begin, end)
    fn get_events(&mut self, filter: EventFilter) -> Result<Vec<models::Event>, StoreError>;
    fn get_event(&mut self, event_id: i32) -> Result<models::Event, StoreError>;
    fn get_event_by_slug(&mut self, slug: &str) -> Result<models::Event, StoreError>;
    fn get_extended_event(
        &mut self,
        auth_token: &AuthToken,
        event_id: EventId,
    ) -> Result<models::ExtendedEvent, StoreError>;
    fn create_event(
        &mut self,
        auth_token: &GlobalAuthToken,
        event: models::ExtendedEvent,
    ) -> Result<EventId, StoreError>;
    fn update_event(
        &mut self,
        auth_token: &AuthToken,
        event: models::ExtendedEvent,
    ) -> Result<(), StoreError>;

    fn delete_event(&mut self, auth_token: &AuthToken, event_id: EventId)
        -> Result<(), StoreError>;

    fn import_event_with_contents(
        &mut self,
        auth_token: &GlobalAuthToken,
        data: models::EventWithContents,
    ) -> Result<EventId, StoreError>;

    /// Get a filtered list of entries of the event
    ///
    /// Entries are returned in chronological order, i.e. sorted by (begin, end)
    fn get_entries_filtered(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
        filter: EntryFilter,
    ) -> Result<Vec<models::FullEntry>, StoreError>;

    fn get_entry(
        &mut self,
        auth_token: &AuthToken,
        entry_id: EntryId,
    ) -> Result<models::FullEntry, StoreError>;
    /// Create a new entry or update the existing entry with the same id.
    ///
    /// If `extend_previous_dates` is true, the previous dates of the (existing) entry are not
    /// replaced with the given ones but instead extended by them.
    ///
    /// If `expected_last_update` is not None, it is checked against the current `last_updated`
    /// value of the entry is checked for equality before updating the entry with the given data. If
    /// it's not equal to the given value, the update is rejected with a `ConcurrentEditConflict`
    /// error If the entity does not exist yet, but `base_version_tag` is given, a `NotExisting`
    /// error is returned.
    ///
    /// # return value
    /// - `Ok(true)` if the entry has been created, successfully
    /// - `Ok(false)` if an existing entry has been updated, successfully
    /// - `Err(StoreError::ConflictEntityExists)` if the entry exists but could not be updated
    ///   (assigned to another event or deleted already)
    /// - `Err(StoreError::ConcurrentEditConflict)` if `expected_last_update` is given but the
    ///   `last_updated` field does not match
    /// - `Err(StoreError::NotExisting)` if `expected_last_update` is given but the entry
    ///   does not exist in the database.
    /// - `Err(_)` if something different went wrong, as usual
    fn create_or_update_entry(
        &mut self,
        auth_token: &AuthToken,
        entry: models::FullNewEntry,
        extend_previous_dates: bool,
        expected_last_update: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<bool, StoreError>;
    fn patch_entry(
        &mut self,
        auth_token: &AuthToken,
        entry_id: EntryId,
        entry_data: models::EntryPatch,
    ) -> Result<(), StoreError>;
    fn delete_entry(
        &mut self,
        auth_token: &AuthToken,
        event_id: EventId,
        entry_id: EntryId,
    ) -> Result<(), StoreError>;
    fn create_or_update_previous_date(
        &mut self,
        auth_token: &AuthToken,
        previous_date: models::FullPreviousDate,
    ) -> Result<bool, StoreError>;
    fn delete_previous_date(
        &mut self,
        auth_token: &AuthToken,
        entry_id: EntryId,
        previous_date_id: PreviousDateId,
    ) -> Result<(), StoreError>;

    fn get_rooms(
        &mut self,
        auth_token: &AuthToken,
        event_id: EventId,
    ) -> Result<Vec<models::Room>, StoreError>;
    /// Create a new room or update the existing entry with the same id.
    ///
    /// # return value
    /// - `Ok(true)` if the room has been created, successfully
    /// - `Ok(false)` if an existing room has been updated, successfully
    /// - `Err(StoreError::ConflictEntityExists)` if the room exists but could not be updated
    ///   (assigned to another event or deleted already)
    /// - `Err(_)` if something different went wrong, as usual
    fn create_or_update_room(
        &mut self,
        auth_token: &AuthToken,
        room: models::NewRoom,
    ) -> Result<bool, StoreError>;
    fn delete_room(
        &mut self,
        auth_token: &AuthToken,
        event_id: EventId,
        room_id: RoomId,
        replace_with_rooms: &[RoomId],
        replace_with_room_comment: &str,
    ) -> Result<(), StoreError>;

    fn get_categories(
        &mut self,
        auth_token: &AuthToken,
        event_id: EventId,
    ) -> Result<Vec<models::Category>, StoreError>;
    /// Create a new category or update the existing entry with the same id.
    ///
    /// # return value
    /// - `Ok(true)` if the category has been created, successfully
    /// - `Ok(false)` if an existing category has been updated, successfully
    /// - `Err(StoreError::ConflictEntityExists)` if the category exists but could not be updated
    ///   (assigned to another event or deleted already)
    /// - `Err(_)` if something different went wrong, as usual
    fn create_or_update_category(
        &mut self,
        auth_token: &AuthToken,
        category: models::NewCategory,
    ) -> Result<bool, StoreError>;
    fn delete_category(
        &mut self,
        auth_token: &AuthToken,
        event_id: EventId,
        category_id: CategoryId,
        replacement_category: Option<CategoryId>,
    ) -> Result<(), StoreError>;

    fn get_announcements(
        &mut self,
        auth_token: &AuthToken,
        event_id: EventId,
        filter: Option<AnnouncementFilter>,
    ) -> Result<Vec<models::FullAnnouncement>, StoreError>;
    /// Create a new announcement or update the existing announcement with the same id.
    ///
    /// # return value
    /// - `Ok(true)` if the announcement has been created, successfully
    /// - `Ok(false)` if an existing announcement has been updated, successfully
    /// - `Err(StoreError::ConflictEntityExists)` if the announcement exists but could not be
    ///   updated (assigned to another event or deleted already)
    /// - `Err(_)` if something different went wrong, as usual
    fn create_or_update_announcement(
        &mut self,
        auth_token: &AuthToken,
        announcement: models::FullNewAnnouncement,
        expected_last_update: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<bool, StoreError>;
    fn patch_announcement(
        &mut self,
        auth_token: &AuthToken,
        announcement_id: AnnouncementId,
        announcement_data: models::AnnouncementPatch,
    ) -> Result<(), StoreError>;
    fn delete_announcement(
        &mut self,
        auth_token: &AuthToken,
        event_id: EventId,
        announcement_id: AnnouncementId,
    ) -> Result<(), StoreError>;

    /// Try to authenticate a client as a new access role for the given event, using the given
    /// passphrase.
    ///
    /// On success, the given session token is updated with the new passphrase id.
    fn authenticate_with_passphrase(
        &mut self,
        event_id: i32,
        passphrase: &str,
        session_token: &mut SessionToken,
    ) -> Result<(), StoreError>;

    /// Get an [AuthToken] instance for a client, representing the client's access roles
    fn get_auth_token_for_session(
        &mut self,
        session_token: &SessionToken,
        event_id: EventId,
    ) -> Result<AuthToken, StoreError>;

    /// Generate a new [SessionToken], derived form the client's existing SessionToken, that is only
    /// authenticated for a single passphrase, which qualifies for the given `expected_privilege`.
    /// The passphrase in the returned SessionToken may be one of the ones from the original
    /// SessionToken or one that can be derived from them.
    fn create_reduced_session_token(
        &mut self,
        client_session_token: &SessionToken,
        event_id: EventId,
        expected_privilege: Privilege,
    ) -> Result<SessionToken, StoreError>;

    /// Create a new passphrase
    ///
    /// returns the id of the new passphrase.
    fn create_passphrase(
        &mut self,
        auth_token: &AuthToken,
        passphrase: models::NewPassphrase,
    ) -> Result<PassphraseId, StoreError>;

    fn delete_passphrase(
        &mut self,
        auth_token: &AuthToken,
        event_id: EventId,
        passphrase_id: PassphraseId,
    ) -> Result<(), StoreError>;

    fn get_passphrases(
        &mut self,
        auth_token: &AuthToken,
        event_id: EventId,
    ) -> Result<Vec<models::Passphrase>, StoreError>;
}

/// Filter options for retrieving entries from the store via KueaPlanStoreFacade::get_entries_filtered()
///
/// Can be constructed through the EntryFilterBuilder
#[derive(Default)]
pub struct EntryFilter {
    /// Filter for entries that end after the given point in time (this includes entries that span
    /// over this point in time)
    pub after: Option<chrono::DateTime<chrono::Utc>>,
    /// If the `after` filter is active: If true, entries that end exactly at the `after` point in
    /// time are included as well. Otherwise, only entries that end strictly after the point in time
    /// are included.
    pub after_inclusive: bool,
    /// Filter for entries that begin before the given point in time (this includes entries that
    /// span over this point in time)
    pub before: Option<chrono::DateTime<chrono::Utc>>,
    /// If the `before` filter is active: If true, entries that start exactly at the `before` point
    /// in time are included as well. Otherwise, only entries that start strictly before the point
    /// in time are included.
    pub before_inclusive: bool,
    /// If true, entries with a previous date that matches the after/before and rooms filters are
    /// included, even if their current begin/end or rooms do not match the after/before filter.
    pub include_previous_date_matches: bool,
    /// Filter for entries that belong to any of the given categories
    pub categories: Option<Vec<uuid::Uuid>>,
    /// Filter for entries that use any of the given rooms
    pub rooms: Option<Vec<uuid::Uuid>>,
    /// If true, filter for entries without any room
    pub no_room: bool,
}

impl EntryFilter {
    pub fn builder() -> EntryFilterBuilder {
        EntryFilterBuilder {
            result: Self::default(),
        }
    }
}

/// Builder for constructing EntryFilter objects
pub struct EntryFilterBuilder {
    result: EntryFilter,
}

impl EntryFilterBuilder {
    /// Add filter, to only include entries that end after the given point in time (this includes
    /// entries that span over this point in time)
    pub fn after(mut self, after: chrono::DateTime<chrono::Utc>, inclusive: bool) -> Self {
        self.result.after = Some(after);
        self.result.after_inclusive = inclusive;
        self
    }
    /// Add filter, to only include entries that begin before the given point in time (this includes
    /// entries that span over this point in time)
    pub fn before(mut self, before: chrono::DateTime<chrono::Utc>, inclusive: bool) -> Self {
        self.result.before = Some(before);
        self.result.before_inclusive = inclusive;
        self
    }
    /// Change after/before filters to include entries which do not cover the specified time
    /// interval but have a previous_date that does.
    pub fn include_previous_date_matches(mut self) -> Self {
        self.result.include_previous_date_matches = true;
        self
    }
    /// Add filter to only include entries that belong to one of the given categories
    pub fn category_is_one_of(mut self, categories: Vec<uuid::Uuid>) -> Self {
        self.result.categories = Some(categories);
        self
    }

    /// Add filter to only include entries that take place (at least) in one of the given rooms
    pub fn in_one_of_these_rooms(mut self, rooms: Vec<uuid::Uuid>) -> Self {
        self.result.rooms = Some(rooms);
        self
    }

    /// Add filter to only include entries that don't have a room assigned
    pub fn without_room(mut self) -> Self {
        self.result.no_room = true;
        self
    }

    /// Create the EntryFilter object
    pub fn build(self) -> EntryFilter {
        self.result
    }
}

/// Filter options for retrieving events from the store via KueaPlanStoreFacade::get_events()
///
/// Can be constructed through the EventFilterBuilder
#[derive(Default)]
pub struct EventFilter {
    /// Filter for events that end at or after the given date (this includes events that span over
    /// this day)
    pub after: Option<chrono::NaiveDate>,
    /// Filter for entries that begin at or before the given date (this includes events that span
    /// over this day)
    pub before: Option<chrono::NaiveDate>,
}

impl EventFilter {
    pub fn builder() -> EventFilterBuilder {
        EventFilterBuilder {
            result: Self::default(),
        }
    }
}

/// Builder for constructing EventFilter objects
pub struct EventFilterBuilder {
    result: EventFilter,
}

impl EventFilterBuilder {
    /// Add filter, to only include events that end at or after the given date (this includes events
    /// that span over this day)
    pub fn after(mut self, after: chrono::NaiveDate) -> Self {
        self.result.after = Some(after);
        self
    }
    /// Add filter, to only include events that start at or before the given date (this includes
    /// events that span over this day)
    pub fn before(mut self, before: chrono::NaiveDate) -> Self {
        self.result.before = Some(before);
        self
    }
    /// Create the EventFilter object
    pub fn build(self) -> EventFilter {
        self.result
    }
}

#[allow(clippy::enum_variant_names)]
pub enum AnnouncementFilter {
    ForDate(chrono::NaiveDate),
    ForCategory(CategoryId),
    ForRoom(RoomId),
}

pub trait KuaPlanStore: Send + Sync {
    fn get_facade<'a>(&'a self) -> Result<Box<dyn KueaPlanStoreFacade + 'a>, StoreError>;
}

#[derive(Debug)]
pub enum StoreError {
    /// Connection the database failed. See string description for details.
    ConnectionError(String),
    /// The query could not be executed because of some error not covered by the other members (see
    /// string description)
    QueryError(diesel::result::Error),
    /// Database transaction could not be commited due to a conflicting concurrent transaction
    TransactionConflict,
    /// The requested entity does not exist
    NotExisting,
    /// The entity could not be created because it already exists, but cannot be updated with the
    /// provided data.
    ConflictEntityExists,
    /// The entity has not been updated because it has been changed since the provided "last
    /// modification date".
    ConcurrentEditConflict,
    /// The client is not authorized for this action. It would need to authenticate for an access
    /// role qualifying for the `required_privilege` on the `event` (or globally if `event_id` is
    /// None).
    PermissionDenied {
        required_privilege: Privilege,
        event_id: Option<EventId>,
    },
    /// The provided data is invalid, i.e. it does not match the expected ranges or violates a
    /// SQL constraint. See string description for details.
    InvalidInputData(String),
    /// Some data queried from the database could not be deserialized. See string description for
    /// details.
    InvalidDataInDatabase(String),
}

impl From<diesel::result::Error> for StoreError {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::NotFound => Self::NotExisting,
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            ) => Self::ConflictEntityExists,
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::SerializationFailure,
                _,
            ) => Self::TransactionConflict,
            diesel::result::Error::DatabaseError(
                e @ diesel::result::DatabaseErrorKind::ForeignKeyViolation
                | e @ diesel::result::DatabaseErrorKind::CheckViolation,
                info,
            ) => Self::InvalidInputData(
                info.constraint_name()
                    .and_then(|constraint_name| {
                        postgres::description_for_postgres_constraint(constraint_name)
                    })
                    .map(|s| s.to_owned())
                    .unwrap_or(format!("{:?}: {}", e, info.message())),
            ),
            diesel::result::Error::SerializationError(e) => Self::InvalidInputData(e.to_string()),
            diesel::result::Error::DeserializationError(e) => {
                Self::InvalidDataInDatabase(e.to_string())
            }
            _ => Self::QueryError(error),
        }
    }
}

impl From<r2d2::Error> for StoreError {
    fn from(error: r2d2::Error) -> Self {
        Self::ConnectionError(error.to_string())
    }
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionError(e) => write!(f, "Error connecting to database: {}", e),
            Self::QueryError(e) => write!(f, "Error while executing database query: {}", e),
            Self::TransactionConflict => f.write_str("Database transaction could not be commited due to a conflicting concurrent transaction"),
            Self::NotExisting => f.write_str("Database record does not exist."),
            Self::ConflictEntityExists => f.write_str("Database record exists already."),
            Self::ConcurrentEditConflict => f.write_str("Updating the entity has been rejected, because the change is not based on the latest version."),
            Self::PermissionDenied {
                required_privilege,
                event_id: Some(event_id),
            } => {
                write!(f, "Client is not authorized to perform this action. {:?} privilege on event {} required.", required_privilege, event_id)
            }
            Self::PermissionDenied {
                required_privilege,
                event_id: None,
            } => {
                write!(f, "Client is not authorized to perform this action. Global {:?} privilege required.", required_privilege)
            }
            Self::InvalidInputData(e) => {
                write!(f, "Data to be stored in database is not valid: {}", e)
            }
            StoreError::InvalidDataInDatabase(e) => {
                write!(f, "Data queried from database could not be deserialized: {}", e)
            }
        }
    }
}

impl std::error::Error for StoreError {}

pub struct EnumMemberNotExistingError {
    pub member_value: i32,
    pub enum_name: &'static str,
}

impl Display for EnumMemberNotExistingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} is not a valid value for {} neum",
            self.member_value, self.enum_name
        )
    }
}
