
use std::env;
use std::fmt::Debug;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;

use crate::models;


pub struct DataStore {
    connection: PgConnection,
}

impl DataStore {
    pub fn new() -> Result<Self, StoreError> {
        return Ok(Self {
            connection: establish_connection()?
        })
    }

    pub fn get_event(&mut self, event_id: i32) -> Result<models::Event, StoreError> {
        use crate::schema::events::dsl::*;

        events
            .filter(id.eq(event_id))
            .first::<models::Event>(&mut self.connection)
            .map_err(|e| e.into())
    }

    pub fn get_entries(&mut self, the_event_id: i32) -> Result<Vec<models::Entry>, StoreError> {
        use crate::schema::entries::dsl::*;

        entries
            .filter(event_id.eq(the_event_id))
            .load::<models::Entry>(&mut self.connection)
            .map_err(|e| e.into())
    }

    pub fn get_entry(&mut self, entry_id: uuid::Uuid) -> Result<models::Entry, StoreError> {
        use crate::schema::entries::dsl::*;

        entries
            .filter(id.eq(entry_id))
            .first::<models::Entry>(&mut self.connection)
            .map_err(|e| e.into())
    }

    pub fn create_entry(&mut self, entry: models::Entry) -> Result<(), StoreError> {
        use crate::schema::entries::dsl::*;

        let count = diesel::insert_into(entries)
            .values(&entry)
            .execute(&mut self.connection)?;
        if count == 0 {Err(StoreError::NotExisting)} else {Ok(())}
    }

    pub fn update_entry(&mut self, entry: models::Entry) -> Result<(), StoreError> {
        use crate::schema::entries::dsl::*;

        let count = diesel::update(entries)
            .filter(id.eq(entry.id))
            .set(&entry)
            .execute(&mut self.connection)?;
        if count == 0 {Err(StoreError::NotExisting)} else {Ok(())}
    }

    pub fn delete_entry(&mut self, entry_id: uuid::Uuid) -> Result<(), StoreError> {
        use crate::schema::entries::dsl::*;

        let count = diesel::delete(entries)
            .filter(id.eq(entry_id))
            .execute(&mut self.connection)?;

        if count == 0 {Err(StoreError::NotExisting)} else {Ok(())}
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


fn establish_connection() -> Result<PgConnection, StoreError> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).map_err(|e| e.into())
}
