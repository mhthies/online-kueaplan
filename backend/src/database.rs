use std::env;
use std::fmt::Debug;

use diesel::PgConnection;

pub mod models;
mod schema;
mod store;

type EventId = i32;
type EntryId = uuid::Uuid;
type RoomId = uuid::Uuid;
type CategoryId = uuid::Uuid;

pub trait KueaPlanStore {
    fn get_event(&mut self, event_id: EventId) -> Result<models::Event, StoreError>;
    fn create_event(&mut self, event: models::Event) -> Result<EventId, StoreError>;

    fn get_entries(&mut self, the_event_id: EventId) -> Result<Vec<models::FullEntry>, StoreError>;
    fn get_entry(&mut self, entry_id: EntryId) -> Result<models::FullEntry, StoreError>;
    fn create_entry(&mut self, entry: models::FullNewEntry) -> Result<(), StoreError>;
    fn update_entry(&mut self, entry: models::FullNewEntry) -> Result<(), StoreError>;
    fn delete_entry(&mut self, entry_id: EntryId) -> Result<(), StoreError>;

    fn get_rooms(&mut self, event_id: EventId) -> Result<Vec<models::Room>, StoreError>;
    fn create_room(&mut self, room: models::NewRoom) -> Result<(), StoreError>;
    fn update_room(&mut self, room: models::NewRoom) -> Result<(), StoreError>;
    fn delete_room(&mut self, room_id: RoomId) -> Result<(), StoreError>;

    fn get_categories(&mut self, event_id: EventId) -> Result<Vec<models::Category>, StoreError>;
    fn create_category(&mut self, category: models::NewCategory) -> Result<(), StoreError>;
    fn update_category(&mut self, category: models::NewCategory) -> Result<(), StoreError>;
    fn delete_category(&mut self, category_id: CategoryId) -> Result<(), StoreError>;
}

pub enum AccessRole {
    User,
    Admin
}

pub struct AuthToken {
    events: Vec<(i32, AccessRole)>,
}

pub trait AuthStore {
    type SessionToken;

    fn create_session() -> Result<Self::SessionToken, StoreError>;
    fn authenticate(event_id: i32, passphrase: &str, session: &Self::SessionToken) -> Result<(), StoreError>;
    fn get_auth_token(session: &Self::SessionToken) -> Result<AuthToken, StoreError>;
    fn logout(session: &Self::SessionToken) -> Result<(), StoreError>;
}

#[derive(Clone)]
pub struct DbPool {
    pool: diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<PgConnection>>,
}

impl DbPool {
    pub fn new() -> Result<Self, String> {
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let connection_manager = diesel::r2d2::ConnectionManager::<PgConnection>::new(database_url);
        Ok(Self {
            pool: diesel::r2d2::Pool::builder()
                .test_on_check_out(true)
                .build(connection_manager)
                .map_err(|e| format!("Could not create database connection pool: {}", e))?,
        })
    }

    pub fn get_store<'a>(&self) -> Result<impl KueaPlanStore + 'a, StoreError> {
        Ok(store::PgDataStore::with_pooled_connection(self.pool.get()?))
    }
}

#[derive(Debug)]
pub enum StoreError {
    ConnectionPoolError(String),
    ConnectionError(diesel::result::ConnectionError),
    QueryError(diesel::result::Error),
    NotExisting,
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
            Self::NotExisting => write!(f, "Database record does not exist."),
        }
    }
}
