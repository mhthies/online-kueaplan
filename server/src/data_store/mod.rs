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
use crate::data_store::auth_token::Privilege;
use auth_token::{AuthToken, EnumMemberNotExistingError, GlobalAuthToken};
use std::env;
use std::fmt::Debug;

pub mod auth_token;
pub mod models;
mod postgres;
mod schema;

#[cfg(test)]
pub mod store_mock;

/// Get a [KuaPlanStore] instances, according the "DATABASE_URL" environment variable.
///
/// The DATABASE_URL must be a PosgreSQL connection url, following the schema
/// "postgres://{user}:{password}@{host}/{database}".
pub fn get_store_from_env() -> Result<impl KuaPlanStore, String> {
    postgres::PgDataStore::new(&env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set")?)
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

    /// Get all entries of the event.
    ///
    /// Entries are returned in chronological order, i.e. sorted by (begin, end)
    fn get_entries(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
    ) -> Result<Vec<models::FullEntry>, StoreError> {
        self.get_entries_filtered(auth_token, the_event_id, EntryFilter::default())
    }

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
    ConnectionPoolError(String),
    ConnectionError(diesel::result::ConnectionError),
    QueryError(diesel::result::Error),
    NotExisting,
    ConflictEntityExists,
    PermissionDenied { required_privilege: Privilege },
    InvalidData,
}

impl From<diesel::result::Error> for StoreError {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::NotFound => Self::NotExisting,
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            ) => Self::ConflictEntityExists,
            _ => Self::QueryError(error),
        }
    }
}

impl From<EnumMemberNotExistingError> for StoreError {
    fn from(_value: EnumMemberNotExistingError) -> Self {
        StoreError::InvalidData
    }
}

impl From<r2d2::Error> for StoreError {
    fn from(error: r2d2::Error) -> Self {
        Self::ConnectionPoolError(error.to_string())
    }
}

impl From<diesel::result::ConnectionError> for StoreError {
    fn from(error: diesel::result::ConnectionError) -> Self {
        Self::ConnectionError(error)
    }
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionPoolError(e) => {
                write!(f, "Could not get database connection from pool: {}", e)
            }
            Self::ConnectionError(e) => write!(f, "Error connecting to database: {}", e),
            Self::QueryError(e) => write!(f, "Error while executing database query: {}", e),
            Self::NotExisting => f.write_str("Database record does not exist."),
            Self::ConflictEntityExists => f.write_str("Database record exists already."),
            Self::PermissionDenied {
                required_privilege: _,
            } => {
                // TODO add list of possible roles to error message
                f.write_str("Client is not authorized to perform this action.")
            }
            Self::InvalidData => {
                f.write_str("Data loaded or stored from/in database is not valid.")
            }
        }
    }
}

impl std::error::Error for StoreError {}
