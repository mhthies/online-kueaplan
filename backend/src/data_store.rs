use std::env;
use std::fmt::Debug;

use crate::auth_session::SessionToken;

pub mod models;
mod schema;
mod postgres;


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

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
#[repr(i32)]
pub enum AccessRole {
    User = 1,
    Orga = 2,
}

impl AccessRole {
    fn implied_roles(&self) -> &'static[AccessRole] {
        match self {
            AccessRole::User => &[],
            AccessRole::Orga => &[AccessRole::User],
        }
    }
}

impl TryFrom<i32> for AccessRole {
    type Error = EnumMemberNotExistingError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(AccessRole::Orga),
            2 => Ok(AccessRole::User),
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
    fn check_privilege(&self, event_id: EventId, privilege_level: AccessRole) -> bool {
        event_id == self.event_id && self.roles.contains(&privilege_level)
    }
}

pub struct AdminAuthToken;

pub trait KuaPlanStore: Send + Sync {
    fn get_facade<'a>(&self) -> Result<Box<dyn KueaPlanStoreFacade + 'a>, StoreError>;
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
