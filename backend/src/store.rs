use std::env;
use std::fmt::Debug;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;

use crate::models;

pub struct DataStore<'a> {
    connection: &'a mut PgConnection,
}

impl<'a> DataStore<'a> {
    pub fn with_connection(connection: &'a mut PgConnection) -> Self {
        return Self { connection };
    }

    pub fn get_event(&mut self, event_id: i32) -> Result<models::Event, StoreError> {
        use crate::schema::events::dsl::*;

        events
            .filter(id.eq(event_id))
            .first::<models::Event>(self.connection)
            .map_err(|e| e.into())
    }

    pub fn get_entries(&mut self, the_event_id: i32) -> Result<Vec<models::FullEntry>, StoreError> {
        use crate::schema::entries::dsl::*;

        self.connection.transaction(|connection| {
            let the_entries = entries
                .filter(event_id.eq(the_event_id))
                .load::<models::Entry>(connection)?;

            let the_entry_rooms = models::EntryRoomMapping::belonging_to(&the_entries)
                .load::<models::EntryRoomMapping>(connection)?
                .grouped_by(&the_entries);

            return Ok(the_entries
                .into_iter()
                .zip(the_entry_rooms.into_iter())
                .map(|(entry, entry_rooms)| models::FullEntry {
                    entry,
                    room_ids: entry_rooms.into_iter().map(|e| e.room_id).collect(),
                })
                .collect());
        })
    }

    pub fn get_entry(&mut self, entry_id: uuid::Uuid) -> Result<models::FullEntry, StoreError> {
        use crate::schema::entries::dsl::*;
        use crate::schema::entry_rooms;

        self.connection.transaction(|connection| {
            let entry = entries
                .filter(id.eq(entry_id))
                .first::<models::Entry>(connection)?;
            let room_ids = entry_rooms::table
                .filter(entry_rooms::dsl::entry_id.eq(entry.id))
                .select(entry_rooms::dsl::room_id)
                .load::<uuid::Uuid>(connection)?;

            Ok(models::FullEntry { entry, room_ids })
        })
    }

    pub fn create_entry(&mut self, entry: models::FullEntry) -> Result<(), StoreError> {
        use crate::schema::entries::dsl::*;

        self.connection.transaction(|connection| {
            diesel::insert_into(entries)
                .values(&entry.entry)
                .execute(connection)?;

            insert_entry_rooms(entry.entry.id, &entry.room_ids, connection)?;

            Ok(())
        })
    }

    pub fn update_entry(&mut self, entry: models::FullEntry) -> Result<(), StoreError> {
        use crate::schema::entries::dsl::*;
        use crate::schema::entry_rooms;

        self.connection.transaction(|connection| {
            let count = diesel::update(entries)
                .filter(id.eq(entry.entry.id))
                .set(&entry.entry)
                .execute(connection)?;
            if count == 0 {
                return Err(StoreError::NotExisting);
            }

            diesel::delete(
                entry_rooms::table.filter(entry_rooms::dsl::entry_id.eq(entry.entry.id)),
            )
            .execute(connection)?;
            insert_entry_rooms(entry.entry.id, &entry.room_ids, connection)?;

            Ok(())
        })
    }

    pub fn delete_entry(&mut self, entry_id: uuid::Uuid) -> Result<(), StoreError> {
        use crate::schema::entries::dsl::*;

        // FIXME we don't want to actually delete but set a 'deleted' flag and update the last-modified timestamp

        let count = diesel::delete(entries)
            .filter(id.eq(entry_id))
            .execute(self.connection)?;
        // entry_room assignments are automatically deleted via CASCADE

        if count == 0 {
            Err(StoreError::NotExisting)
        } else {
            Ok(())
        }
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

fn insert_entry_rooms(
    the_entry_id: uuid::Uuid,
    room_ids: &Vec<uuid::Uuid>,
    connection: &mut PgConnection,
) -> Result<(), diesel::result::Error> {
    use crate::schema::entry_rooms::dsl::*;

    diesel::insert_into(entry_rooms)
        .values(
            room_ids
                .iter()
                .map(|the_room_id| (entry_id.eq(the_entry_id), room_id.eq(the_room_id)))
                .collect::<Vec<_>>(),
        )
        .execute(connection)
        .map(|_| ())
}

fn establish_connection() -> Result<PgConnection, StoreError> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).map_err(|e| e.into())
}
