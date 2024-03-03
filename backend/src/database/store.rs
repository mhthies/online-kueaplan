use diesel::pg::PgConnection;
use diesel::prelude::*;

use super::{models, schema, KueaPlanStore, StoreError};

pub struct PgDataStore {
    connection: diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<PgConnection>>,
}

impl PgDataStore {
    pub fn with_pooled_connection(
        connection: diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<PgConnection>>,
    ) -> Self {
        return Self { connection };
    }
}

impl KueaPlanStore for PgDataStore {
    fn get_event(&mut self, event_id: i32) -> Result<models::Event, StoreError> {
        use schema::events::dsl::*;

        events
            .filter(id.eq(event_id))
            .first::<models::Event>(&mut self.connection)
            .map_err(|e| e.into())
    }
    
    fn create_event(&mut self, event: models::Event) -> Result<i32, StoreError> {
        todo!()
    }

    fn get_entries(&mut self, the_event_id: i32) -> Result<Vec<models::FullEntry>, StoreError> {
        use schema::entries::dsl::*;

        self.connection.transaction(|connection| {
            let the_entries = entries
                .filter(event_id.eq(the_event_id))
                .filter(deleted.eq(false))
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

    fn get_entry(&mut self, entry_id: uuid::Uuid) -> Result<models::FullEntry, StoreError> {
        use schema::entries::dsl::*;
        use schema::entry_rooms;

        self.connection.transaction(|connection| {
            let entry = entries
                .filter(id.eq(entry_id))
                .first::<models::Entry>(connection)?;

            if entry.deleted {
                return Err(StoreError::NotExisting);
            }

            let room_ids = entry_rooms::table
                .filter(entry_rooms::dsl::entry_id.eq(entry.id))
                .select(entry_rooms::dsl::room_id)
                .load::<uuid::Uuid>(connection)?;

            Ok(models::FullEntry { entry, room_ids })
        })
    }

    fn create_entry(&mut self, entry: models::FullNewEntry) -> Result<(), StoreError> {
        use schema::entries::dsl::*;

        self.connection.transaction(|connection| {
            diesel::insert_into(entries)
                .values(&entry.entry)
                .execute(connection)?;

            insert_entry_rooms(entry.entry.id, &entry.room_ids, connection)?;

            Ok(())
        })
    }

    fn update_entry(&mut self, entry: models::FullNewEntry) -> Result<(), StoreError> {
        use schema::entries::dsl::*;
        use schema::entry_rooms;

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

    fn delete_entry(&mut self, entry_id: uuid::Uuid) -> Result<(), StoreError> {
        use schema::entries::dsl::*;

        // FIXME we don't want to actually delete but set a 'deleted' flag and update the last-modified timestamp
        
        let count = diesel::update(entries)
                .filter(id.eq(entry_id))
                .set(deleted.eq(false))
                .execute(&mut self.connection)?;
        if count == 0 {
            return Err(StoreError::NotExisting);
        }

        Ok(())
    }
    
    fn get_rooms(&mut self, event_id: i32) -> Result<Vec<models::Room>, StoreError> {
        todo!()
    }
    
    fn create_room(&mut self, room: models::NewRoom) -> Result<(), StoreError> {
        todo!()
    }
    
    fn update_room(&mut self, room: models::NewRoom) -> Result<(), StoreError> {
        todo!()
    }
    
    fn delete_room(&mut self, room_id: uuid::Uuid) -> Result<(), StoreError> {
        todo!()
    }
    
    fn get_categories(&mut self, event_id: i32) -> Result<Vec<models::Category>, StoreError> {
        todo!()
    }
    
    fn create_category(&mut self, category: models::NewCategory) -> Result<(), StoreError> {
        todo!()
    }
    
    fn update_category(&mut self, category: models::NewCategory) -> Result<(), StoreError> {
        todo!()
    }
    
    fn delete_category(&mut self, category_id: uuid::Uuid) -> Result<(), StoreError> {
        todo!()
    }
}

fn insert_entry_rooms(
    the_entry_id: uuid::Uuid,
    room_ids: &Vec<uuid::Uuid>,
    connection: &mut PgConnection,
) -> Result<(), diesel::result::Error> {
    use schema::entry_rooms::dsl::*;

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
