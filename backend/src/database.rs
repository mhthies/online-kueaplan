use std::env;
use std::fmt::Debug;

use diesel::PgConnection;

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
