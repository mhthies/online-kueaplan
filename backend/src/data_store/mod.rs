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

use std::env;
use std::fmt::Debug;

use crate::auth_session::SessionToken;
use crate::CliAuthToken;

pub mod models;
mod schema;
mod postgres;

#[cfg(test)]
pub mod store_mock;

/// Get a [KuaPlanStore] instances, according the "DATABASE_URL" environment variable.
/// 
/// The DATABASE_URL must be a PosgreSQL connection url, following the schema
/// "postgres://{user}:{password}@{host}/{database}".
pub fn get_store_from_env() -> Result<impl KuaPlanStore, String> {
    Ok(postgres::PgDataStore::new(&env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set")?)?)
}


pub type EventId = i32;
pub type EntryId = uuid::Uuid;
pub type RoomId = uuid::Uuid;
pub type CategoryId = uuid::Uuid;
pub type PassphraseId = i32;

pub trait KueaPlanStoreFacade {
    fn get_event(&mut self, auth_token: &AuthToken, event_id: EventId) -> Result<models::Event, StoreError>;
    fn create_event(&mut self, auth_token: &AdminAuthToken, event: models::NewEvent) -> Result<EventId, StoreError>;

    fn get_entries(&mut self, auth_token: &AuthToken, the_event_id: EventId) -> Result<Vec<models::FullEntry>, StoreError>;
    fn get_entry(&mut self, auth_token: &AuthToken, entry_id: EntryId) -> Result<models::FullEntry, StoreError>;
    fn create_entry(&mut self, auth_token: &AuthToken, entry: models::FullNewEntry) -> Result<(), StoreError>;
    fn update_entry(&mut self, auth_token: &AuthToken, entry: models::FullNewEntry) -> Result<(), StoreError>;
    fn delete_entry(&mut self, auth_token: &AuthToken, event_id: EventId, entry_id: EntryId) -> Result<(), StoreError>;

    fn get_rooms(&mut self, auth_token: &AuthToken, event_id: EventId) -> Result<Vec<models::Room>, StoreError>;
    fn create_room(&mut self, auth_token: &AuthToken, room: models::NewRoom) -> Result<(), StoreError>;
    fn update_room(&mut self, auth_token: &AuthToken, room: models::NewRoom) -> Result<(), StoreError>;
    fn delete_room(&mut self, auth_token: &AuthToken, event_id: EventId, room_id: RoomId) -> Result<(), StoreError>;

    fn get_categories(&mut self, auth_token: &AuthToken, event_id: EventId) -> Result<Vec<models::Category>, StoreError>;
    fn create_category(&mut self, auth_token: &AuthToken, category: models::NewCategory) -> Result<(), StoreError>;
    fn update_category(&mut self, auth_token: &AuthToken, category: models::NewCategory) -> Result<(), StoreError>;
    fn delete_category(&mut self, auth_token: &AuthToken, event_id: EventId, category_id: CategoryId) -> Result<(), StoreError>;

    /**
     * Try to authorize for a new privilege level for the given event, using the given passphrase.
     *
     * On success, the given session token is updated with the new passphrase id.
     */
    fn authorize(
        &mut self,
        event_id: i32,
        passphrase: &str,
        session_token: &mut SessionToken,
    ) -> Result<(), StoreError>;
    fn check_authorization(&mut self, session_token: &SessionToken, event_id: EventId) -> Result<AuthToken, StoreError>;
}

/// Possible roles, a single user can have with respect to a certain event
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
#[repr(i32)]
pub enum AccessRole {
    User = 1,
    Orga = 2,
    Admin = 3,
}

impl AccessRole {
    /// Get a list of roles, which are implicitly granted to a user who was authorized to this role.
    fn implied_roles(&self) -> &'static[AccessRole] {
        match self {
            AccessRole::User => &[],
            AccessRole::Orga => &[AccessRole::User],
            AccessRole::Admin => &[AccessRole::Orga, AccessRole::User],
        }
    }
}

impl TryFrom<i32> for AccessRole {
    type Error = EnumMemberNotExistingError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(AccessRole::User),
            2 => Ok(AccessRole::Orga),
            3 => Ok(AccessRole::Admin),
            _ => Err(EnumMemberNotExistingError{}),
        }
    }
}

pub struct EnumMemberNotExistingError;

pub struct AuthToken {
    event_id: i32,
    roles: Vec<AccessRole>,
}

impl AuthToken {
    fn check_privilege(&self, event_id: EventId, privilege_level: AccessRole) -> Result<(), StoreError> {
        if event_id == self.event_id && self.roles.contains(&privilege_level) {
            Ok(())
        } else {
            Err(StoreError::PermissionDenied)
        }
    }

    pub fn get_cli_authorization(_token: &CliAuthToken, event_id: EventId) -> Self {
        let mut roles = vec![AccessRole::Admin];
        roles.extend(AccessRole::Admin.implied_roles());
        AuthToken {
            event_id,
            roles,
        }
    }
}

pub struct AdminAuthToken {
    roles: Vec<AccessRole>,
}

impl AdminAuthToken {
    fn check_privilege(&self, privilege_level: AccessRole) -> Result<(), StoreError> {
        if self.roles.contains(&privilege_level) {
            Ok(())
        } else {
            Err(StoreError::PermissionDenied)
        }
    }

    pub fn get_global_cli_authorization(_token: &CliAuthToken) -> Self {
        let mut roles = vec![AccessRole::Admin];
        roles.extend(AccessRole::Admin.implied_roles());
        AdminAuthToken {
            roles,
        }
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
    PermissionDenied,
    InvalidSession,
    InvalidData,
}

impl From<diesel::result::Error> for StoreError {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::NotFound => Self::NotExisting,
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
        return Self::ConnectionPoolError(error.to_string());
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
            Self::PermissionDenied => f.write_str("Client is not authorized to perform this action"),
            Self::InvalidSession => f.write_str("Session token provided by client or session data is invalid"),
            Self::InvalidData => f.write_str("Data loaded or stored from/in database is not valid."),
        }
    }
}
