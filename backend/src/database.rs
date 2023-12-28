
use std::fmt::Debug;

pub mod models;
mod schema;
mod store;

pub trait KueaPlanStore {
    fn get_event(&mut self, event_id: i32) -> Result<models::Event, StoreError>;
    fn get_entries(&mut self, the_event_id: i32) -> Result<Vec<models::FullEntry>, StoreError>;
    fn get_entry(&mut self, entry_id: uuid::Uuid) -> Result<models::FullEntry, StoreError>;
    fn create_entry(&mut self, entry: models::FullEntry) -> Result<(), StoreError>;
    fn update_entry(&mut self, entry: models::FullEntry) -> Result<(), StoreError>;
    fn delete_entry(&mut self, entry_id: uuid::Uuid) -> Result<(), StoreError>;
}

pub fn get_pg_store<'a>(connection: &'a mut diesel::pg::PgConnection) -> impl KueaPlanStore + 'a {
    store::PgDataStore::with_connection(connection)
}


#[derive(Debug)]
pub enum StoreError {
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

impl From<diesel::result::ConnectionError> for StoreError {
    fn from(error: diesel::result::ConnectionError) -> Self {
        Self::ConnectionError(error)
    }
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionError(e) => write!(f, "Error connecting to database: {}", e),
            Self::QueryError(e) => write!(f, "Error while executing database query: {}", e),
            Self::NotExisting => write!(f, "Database record does not exist."),
        }
    }
}
