use std::fmt::Debug;

use diesel::PgConnection;
use crate::auth_session::SessionToken;

pub mod models;
mod schema;
mod postgres;

type EventId = i32;
type EntryId = uuid::Uuid;
type RoomId = uuid::Uuid;
type CategoryId = uuid::Uuid;
pub type PassphraseId = i32;

pub trait KueaPlanStore {
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
}

pub trait AuthStore {
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
    fn logout(&mut self, session: &str) -> Result<(), StoreError>;
}

#[derive(Eq, PartialEq)]
pub enum AccessRole {
    User,
    Orga,
}

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

#[derive(Clone)]
pub struct DbPool {
    pool: diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<PgConnection>>,
}

impl DbPool {
    pub fn new(database_url: &str) -> Result<Self, String> {
        let connection_manager = diesel::r2d2::ConnectionManager::<PgConnection>::new(database_url);
        Ok(Self {
            pool: diesel::r2d2::Pool::builder()
                .test_on_check_out(true)
                .build(connection_manager)
                .map_err(|e| format!("Could not create database connection pool: {}", e))?,
        })
    }

    pub fn get_store<'a>(&self) -> Result<impl KueaPlanStore + AuthStore + 'a, StoreError> {
        Ok(postgres::PgDataStore::with_pooled_connection(self.pool.get()?))
    }
}

#[derive(Debug)]
pub enum StoreError {
    ConnectionPoolError(String),
    ConnectionError(diesel::result::ConnectionError),
    QueryError(diesel::result::Error),
    NotExisting,
    PermissionDenied,
    InvalidSession,
}

impl From<diesel::result::Error> for StoreError {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::NotFound => Self::NotExisting,
            _ => Self::QueryError(error),
        }
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
        }
    }
}
