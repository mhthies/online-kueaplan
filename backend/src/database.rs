use std::env;
use std::fmt::Debug;

use diesel::PgConnection;
use dotenvy::dotenv;

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
    pub fn new() -> Self {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let connection_manager = diesel::r2d2::ConnectionManager::<PgConnection>::new(database_url);
        Self {
            pool: diesel::r2d2::Pool::builder()
                .test_on_check_out(true)
                .build(connection_manager)
                .expect("Could not build connection pool"),
        }
    }

    pub fn get_store<'a>(&self) -> impl KueaPlanStore + 'a {
        // TODO better error handling
        store::PgDataStore::with_pooled_connection(
            self.pool
                .get()
                .expect("couldn't get db connection from pool"),
        )
    }
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
