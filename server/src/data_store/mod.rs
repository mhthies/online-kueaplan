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
//! There is also a mock implementation for unittests. Other [KueaPlanStore] implementations may be
//! added later and selected via the "DATABASE_URL" environment variable.

use crate::auth_session::SessionToken;
use crate::cli_error::CliError;
use crate::cli_error::CliError::UnexpectedStoreError;
use crate::data_store::auth_token::Privilege;
use crate::setup;
use auth_token::{AuthToken, GlobalAuthToken};
use std::fmt::Debug;

pub mod auth_token;
pub mod models;
mod postgres;
mod schema;

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
pub type RoomId = uuid::Uuid;
pub type CategoryId = uuid::Uuid;
pub type PassphraseId = i32;

pub trait KueaPlanStoreFacade {
    fn get_event(
        &mut self,
        auth_token: &AuthToken,
        event_id: EventId,
    ) -> Result<models::Event, StoreError>;
    fn create_event(
        &mut self,
        auth_token: &GlobalAuthToken,
        event: models::NewEvent,
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
    /// # return value
    /// - `Ok(true)` if the entry has been created, successfully
    /// - `Ok(false)` if an existing entry has been updated, successfully
    /// - `Err(StoreError::ConflictEntityExists)` if the entry exists but could not be updated
    ///   (assigned to another event or deleted already)
    /// - `Err(_)` if something different went wrong, as usual
    fn create_or_update_entry(
        &mut self,
        auth_token: &AuthToken,
        entry: models::FullNewEntry,
        extend_previous_dates: bool,
    ) -> Result<bool, StoreError>;
    fn delete_entry(
        &mut self,
        auth_token: &AuthToken,
        event_id: EventId,
        entry_id: EntryId,
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
}

/// Filter options for retrieving entries from the store via KueaPlanStoreFacade::get_entries_filtered()
///
/// Can be constructed through the EntryFilterBuilder
#[derive(Default)]
pub struct EntryFilter {
    /// Filter for entries that end after the given point in time (this includes entries that span
    /// over this point in time)
    pub after: Option<chrono::DateTime<chrono::Utc>>,
    /// Filter for entries that begin before the given point in time (this includes entries that
    /// span over this point in time)
    pub before: Option<chrono::DateTime<chrono::Utc>>,
    /// Filter for entries that belong to any of the given categories
    pub categories: Option<Vec<uuid::Uuid>>,
    /// Filter for entries that use any of the given rooms
    pub rooms: Option<Vec<uuid::Uuid>>,
    /// If true, filter for entries without any room
    pub no_room: bool,
}

impl EntryFilter {
    /// Checks if a given entry matches the filter
    ///
    /// Usually, filtering should be done by the database. This function can be used for separate
    /// checks of individual entries in software.
    pub fn matches(&self, entry: &models::FullEntry) -> bool {
        if let Some(after) = self.after {
            if after >= entry.entry.end {
                return false;
            }
        }
        if let Some(before) = self.before {
            if before <= entry.entry.begin {
                return false;
            }
        }
        if let Some(categories) = &self.categories {
            if !categories.contains(&entry.entry.category) {
                return false;
            }
        }
        if let Some(rooms) = &self.rooms {
            if !rooms.iter().any(|r| entry.room_ids.contains(r)) {
                return false;
            }
        }
        if self.no_room && !entry.room_ids.is_empty() {
            return false;
        }
        true
    }
}

/// Builder for constructing EntryFilter objects
pub struct EntryFilterBuilder {
    result: EntryFilter,
}

impl EntryFilterBuilder {
    pub fn new() -> Self {
        Self {
            result: EntryFilter {
                after: None,
                before: None,
                categories: None,
                rooms: None,
                no_room: false,
            },
        }
    }

    /// Add filter, to only include entries that end after the given point in time (this includes
    /// entries that span over this point in time)
    pub fn after(&mut self, after: chrono::DateTime<chrono::Utc>) -> &mut Self {
        self.result.after = Some(after);
        self
    }
    /// Add filter, to only include entries that begin before the given point in time (this includes
    /// entries that span over this point in time)
    pub fn before(&mut self, before: chrono::DateTime<chrono::Utc>) -> &mut Self {
        self.result.before = Some(before);
        self
    }
    /// Add filter to only include entries that belong to one of the given categories
    pub fn category_is_one_of(&mut self, categories: Vec<uuid::Uuid>) -> &mut Self {
        self.result.categories = Some(categories);
        self
    }

    /// Add filter to only include entries that take place (at least) in one of the given rooms
    pub fn in_one_of_these_rooms(&mut self, rooms: Vec<uuid::Uuid>) -> &mut Self {
        self.result.rooms = Some(rooms);
        self
    }

    /// Add filter to only include entries that don't have a room assigned
    pub fn without_room(&mut self) -> &mut Self {
        self.result.no_room = true;
        self
    }

    /// Create the EntryFilter object
    pub fn build(self) -> EntryFilter {
        self.result
    }
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
                _,
            ) => Self::InvalidInputData(format!("{:?}", e)),
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
            },
        }
    }
}

impl std::error::Error for StoreError {}
