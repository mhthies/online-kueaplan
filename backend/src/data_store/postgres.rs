use std::str::FromStr;

use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::data_store::AccessRole;

use super::{models, schema, AuthStore, AuthToken, EventId, KueaPlanStore, StoreError};

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
    fn get_event(
        &mut self,
        auth_token: &AuthToken,
        event_id: i32,
    ) -> Result<models::Event, StoreError> {
        use schema::events::dsl::*;

        if !auth_token.check_admin_privilege() {
            return Err(StoreError::PermissionDenied);
        }

        events
            .filter(id.eq(event_id))
            .first::<models::Event>(&mut self.connection)
            .map_err(|e| e.into())
    }

    fn create_event(
        &mut self,
        auth_token: &AuthToken,
        event: models::NewEvent,
    ) -> Result<i32, StoreError> {
        use schema::events::dsl::*;

        if !auth_token.check_admin_privilege() {
            return Err(StoreError::PermissionDenied);
        }

        Ok(diesel::insert_into(events)
            .values(&event)
            .get_results::<models::Event>(&mut self.connection)
            .map(|e| e[0].id)?)
    }

    fn get_entries(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: i32,
    ) -> Result<Vec<models::FullEntry>, StoreError> {
        use schema::entries::dsl::*;

        if !auth_token.check_event_privilege(the_event_id, AccessRole::User) {
            return Err(StoreError::PermissionDenied);
        }

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

    fn get_entry(
        &mut self,
        auth_token: &AuthToken,
        entry_id: uuid::Uuid,
    ) -> Result<models::FullEntry, StoreError> {
        use schema::entries::dsl::*;
        use schema::entry_rooms;

        self.connection.transaction(|connection| {
            let entry = entries
                .filter(id.eq(entry_id))
                .first::<models::Entry>(connection)?;
            if !auth_token.check_event_privilege(entry.event_id, AccessRole::User) {
                return Err(StoreError::PermissionDenied);
            }

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

    fn create_entry(
        &mut self,
        auth_token: &AuthToken,
        entry: models::FullNewEntry,
    ) -> Result<(), StoreError> {
        use schema::entries::dsl::*;

        if !auth_token.check_event_privilege(entry.entry.event_id, AccessRole::Orga) {
            return Err(StoreError::PermissionDenied);
        }

        self.connection.transaction(|connection| {
            diesel::insert_into(entries)
                .values(&entry.entry)
                .execute(connection)?;

            insert_entry_rooms(entry.entry.id, &entry.room_ids, connection)?;

            Ok(())
        })
    }

    fn update_entry(
        &mut self,
        auth_token: &AuthToken,
        entry: models::FullNewEntry,
    ) -> Result<(), StoreError> {
        use schema::entries::dsl::*;
        use schema::entry_rooms;

        // The event_id of the existing entry is ensured to be the same (see below), so the
        // privilege level check holds for the existing and the new entry.
        if !auth_token.check_event_privilege(entry.entry.event_id, AccessRole::Orga) {
            return Err(StoreError::PermissionDenied);
        }

        self.connection.transaction(|connection| {
            let count = diesel::update(entries)
                .filter(id.eq(entry.entry.id))
                // By limiting the search of existing entries to the same event, we prevent changes
                // of the event id (i.e. "moving" entries between events), which would be a security
                // loop hole
                .filter(event_id.eq(entry.entry.event_id))
                .filter(deleted.eq(false))
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

    fn delete_entry(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
        entry_id: uuid::Uuid,
    ) -> Result<(), StoreError> {
        use schema::entries::dsl::*;

        // The correctness of the given event_id is checked in the DELETE statement below
        if !auth_token.check_event_privilege(the_event_id, AccessRole::Orga) {
            return Err(StoreError::PermissionDenied);
        }

        let count = diesel::update(entries)
            .filter(id.eq(entry_id))
            .filter(event_id.eq(the_event_id))
            .set(deleted.eq(true))
            .execute(&mut self.connection)?;
        if count == 0 {
            return Err(StoreError::NotExisting);
        }

        Ok(())
    }

    fn get_rooms(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: i32,
    ) -> Result<Vec<models::Room>, StoreError> {
        use schema::rooms::dsl::*;

        if !auth_token.check_event_privilege(the_event_id, AccessRole::User) {
            return Err(StoreError::PermissionDenied);
        }

        Ok(rooms
            .select(models::Room::as_select())
            .filter(event_id.eq(the_event_id))
            .filter(deleted.eq(false))
            .load::<models::Room>(&mut self.connection)?)
    }

    fn create_room(
        &mut self,
        auth_token: &AuthToken,
        room: models::NewRoom,
    ) -> Result<(), StoreError> {
        use schema::rooms::dsl::*;

        if !auth_token.check_event_privilege(room.event_id, AccessRole::Orga) {
            return Err(StoreError::PermissionDenied);
        }

        diesel::insert_into(rooms)
            .values(&room)
            .execute(&mut self.connection)?;

        Ok(())
    }

    fn update_room(
        &mut self,
        auth_token: &AuthToken,
        room: models::NewRoom,
    ) -> Result<(), StoreError> {
        use schema::rooms::dsl::*;

        // The event_id of the existing room is ensured to be the same (see below), so the
        // privilege level check holds for both, the existing and the new room.
        if !auth_token.check_event_privilege(room.event_id, AccessRole::Orga) {
            return Err(StoreError::PermissionDenied);
        }

        let count = diesel::update(rooms)
            .filter(id.eq(room.id))
            // By limiting the search of existing rooms to the same event, we prevent changes
            // of the event id (i.e. "moving" entries between events), which would be a security
            // loop hole
            .filter(event_id.eq(room.event_id))
            .filter(deleted.eq(false))
            .set(&room)
            .execute(&mut self.connection)?;
        if count == 0 {
            return Err(StoreError::NotExisting);
        }
        Ok(())
    }

    fn delete_room(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
        room_id: uuid::Uuid,
    ) -> Result<(), StoreError> {
        use schema::rooms::dsl::*;

        // The correctness of the given event_id is checked in the DELETE statement below
        if !auth_token.check_event_privilege(the_event_id, AccessRole::Orga) {
            return Err(StoreError::PermissionDenied);
        }

        let count = diesel::update(rooms)
            .filter(id.eq(room_id))
            .filter(event_id.eq(the_event_id))
            .set(deleted.eq(true))
            .execute(&mut self.connection)?;
        if count == 0 {
            return Err(StoreError::NotExisting);
        }

        Ok(())
    }

    fn get_categories(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: i32,
    ) -> Result<Vec<models::Category>, StoreError> {
        use schema::categories::dsl::*;

        if !auth_token.check_event_privilege(the_event_id, AccessRole::User) {
            return Err(StoreError::PermissionDenied);
        }

        Ok(categories
            .select(models::Category::as_select())
            .filter(event_id.eq(the_event_id))
            .filter(deleted.eq(false))
            .load::<models::Category>(&mut self.connection)?)
    }

    fn create_category(
        &mut self,
        auth_token: &AuthToken,
        category: models::NewCategory,
    ) -> Result<(), StoreError> {
        use schema::categories::dsl::*;

        if !auth_token.check_event_privilege(category.event_id, AccessRole::Orga) {
            return Err(StoreError::PermissionDenied);
        }

        diesel::insert_into(categories)
            .values(&category)
            .execute(&mut self.connection)?;

        Ok(())
    }

    fn update_category(
        &mut self,
        auth_token: &AuthToken,
        category: models::NewCategory,
    ) -> Result<(), StoreError> {
        use schema::categories::dsl::*;

        if !auth_token.check_event_privilege(category.event_id, AccessRole::Orga) {
            return Err(StoreError::PermissionDenied);
        }

        let count = diesel::update(categories)
            .filter(id.eq(category.id))
            // By limiting the search of existing categories to the same event, we prevent changes
            // of the event id (i.e. "moving" categories between events), which would be a security
            // loop hole
            .filter(event_id.eq(category.event_id))
            .filter(deleted.eq(false))
            .set(&category)
            .execute(&mut self.connection)?;
        if count == 0 {
            return Err(StoreError::NotExisting);
        }
        Ok(())
    }

    fn delete_category(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
        category_id: uuid::Uuid,
    ) -> Result<(), StoreError> {
        use schema::categories::dsl::*;

        // The correctness of the given event_id is checked in the DELETE statement below
        if !auth_token.check_event_privilege(the_event_id, AccessRole::Orga) {
            return Err(StoreError::PermissionDenied);
        }

        let count = diesel::update(categories)
            .filter(id.eq(category_id))
            .filter(event_id.eq(the_event_id))
            .set(deleted.eq(true))
            .execute(&mut self.connection)?;
        if count == 0 {
            return Err(StoreError::NotExisting);
        }

        Ok(())
    }
}

impl AuthStore for PgDataStore {
    fn create_session(&mut self) -> Result<String, StoreError> {
        todo!()
    }

    fn authenticate(
        &mut self,
        event_id: i32,
        passphrase: &str,
        session: &str,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_auth_token(&mut self, session: &str) -> Result<AuthToken, StoreError> {
        todo!()
    }

    fn logout(&mut self, session: &str) -> Result<(), StoreError> {
        todo!()
    }
}

type SessionId = [u8; 128];
struct PgSessionToken {
    session_id: SessionId
}

impl FromStr for PgSessionToken {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!("Convert from base64")
    }
}

impl ToString for PgSessionToken {
    fn to_string(&self) -> String {
        todo!("Convert to base64")
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
