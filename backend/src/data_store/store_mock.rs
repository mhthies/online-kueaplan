use crate::auth_session::SessionToken;
use crate::data_store::models::{
    Category, Event, FullEntry, FullNewEntry, NewCategory, NewEvent, NewRoom, Room,
};
use crate::data_store::{
    models, AccessRole, GlobalAuthToken, AuthToken, EventId, KuaPlanStore, KueaPlanStoreFacade,
    StoreError,
};
use std::sync::Mutex;

/**
 * A mock [KuaPlanStore] implementation for testing.
 *
 * The simulated database consists of the [StoreMockData] structure with vectors of entities. These
 * can be directly modified by the tests.
 *
 * Except from checking for entity existence, the interface functions of this mock don't do any
 * error or privilege checking. Instead, the [StoreMockData.next_error] attribute can be set to
 * simulate a database error.
 */
#[derive(Default)]
pub struct StoreMock {
    pub data: Mutex<StoreMockData>,
}

impl KuaPlanStore for StoreMock {
    fn get_facade<'a>(&'a self) -> Result<Box<dyn KueaPlanStoreFacade + 'a>, StoreError> {
        Ok(Box::new(StoreMockFacade { store: self }))
    }
}

#[derive(Default)]
pub struct StoreMockData {
    pub event: Option<Event>,
    pub entries: Vec<FullEntry>,
    pub rooms: Vec<Room>,
    pub categories: Vec<Category>,
    /// If not none, the next call to a store facade method will return this error.
    pub next_error: Option<StoreError>,
}

struct StoreMockFacade<'a> {
    store: &'a StoreMock,
}

impl<'a> crate::data_store::KueaPlanStoreFacade for StoreMockFacade<'a> {
    fn get_event(
        &mut self,
        _auth_token: &AuthToken,
        _event_id: crate::data_store::EventId,
    ) -> Result<Event, StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        if data.event.is_none() {
            Err(StoreError::NotExisting)
        } else {
            Ok(data.event.clone().unwrap())
        }
    }

    fn create_event(
        &mut self,
        _auth_token: &GlobalAuthToken,
        event: NewEvent,
    ) -> Result<crate::data_store::EventId, StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        let event_id = 42;
        data.event.replace(models::Event {
            id: event_id,
            begin_date: event.begin_date,
            end_date: event.end_date,
            title: event.title,
        });
        Ok(event_id)
    }

    fn get_entries(
        &mut self,
        _auth_token: &AuthToken,
        _the_event_id: crate::data_store::EventId,
    ) -> Result<Vec<FullEntry>, StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        Ok(data.entries.clone())
    }

    fn get_entry(
        &mut self,
        _auth_token: &AuthToken,
        entry_id: crate::data_store::EntryId,
    ) -> Result<FullEntry, StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        data.entries
            .iter()
            .filter(|e| e.entry.id == entry_id)
            .next()
            .map(|e| e.clone())
            .ok_or(StoreError::NotExisting)
    }

    fn create_entry(
        &mut self,
        _auth_token: &AuthToken,
        entry: FullNewEntry,
    ) -> Result<(), StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        data.entries.push(models::FullEntry {
            entry: models::Entry {
                id: entry.entry.id,
                title: entry.entry.title,
                description: entry.entry.description,
                responsible_person: entry.entry.responsible_person,
                is_room_reservation: entry.entry.is_room_reservation,
                residue_of: entry.entry.residue_of,
                event_id: entry.entry.event_id,
                begin: entry.entry.begin,
                end: entry.entry.end,
                category: entry.entry.category,
                deleted: false,
                last_updated: chrono::Utc::now(),
                comment: entry.entry.comment,
                room_comment: entry.entry.room_comment,
                time_comment: entry.entry.time_comment,
                is_exclusive: entry.entry.is_exclusive,
                is_cancelled: entry.entry.is_cancelled,
            },
            room_ids: entry.room_ids,
        });
        Ok(())
    }

    fn update_entry(
        &mut self,
        _auth_token: &AuthToken,
        entry: FullNewEntry,
    ) -> Result<(), StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        let existing_entry = data
            .entries
            .iter_mut()
            .filter(|e| e.entry.id == entry.entry.id)
            .next()
            .ok_or(StoreError::NotExisting)?;
        existing_entry.entry.title = entry.entry.title;
        existing_entry.entry.description = entry.entry.description;
        existing_entry.entry.responsible_person = entry.entry.responsible_person;
        existing_entry.entry.is_room_reservation = entry.entry.is_room_reservation;
        existing_entry.entry.residue_of = entry.entry.residue_of;
        existing_entry.entry.event_id = entry.entry.event_id;
        existing_entry.entry.begin = entry.entry.begin;
        existing_entry.entry.end = entry.entry.end;
        existing_entry.entry.category = entry.entry.category;
        existing_entry.entry.last_updated = chrono::Utc::now();
        existing_entry.entry.comment = entry.entry.comment;
        existing_entry.entry.room_comment = entry.entry.room_comment;
        existing_entry.entry.time_comment = entry.entry.time_comment;
        existing_entry.entry.is_exclusive = entry.entry.is_exclusive;
        existing_entry.entry.is_cancelled = entry.entry.is_cancelled;
        existing_entry.room_ids = entry.room_ids;
        Ok(())
    }

    fn delete_entry(
        &mut self,
        _auth_token: &AuthToken,
        _event_id: crate::data_store::EventId,
        entry_id: crate::data_store::EntryId,
    ) -> Result<(), StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        data.entries.retain(|e| e.entry.id != entry_id);
        Ok(())
    }

    fn get_rooms(
        &mut self,
        _auth_token: &AuthToken,
        _event_id: crate::data_store::EventId,
    ) -> Result<Vec<Room>, StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        Ok(data.rooms.clone())
    }

    fn create_room(&mut self, _auth_token: &AuthToken, room: NewRoom) -> Result<(), StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        data.rooms.push(Room {
            id: room.id,
            title: room.title,
            description: room.description,
            event_id: room.event_id,
        });
        Ok(())
    }

    fn update_room(&mut self, _auth_token: &AuthToken, room: NewRoom) -> Result<(), StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        let existing_room = data
            .rooms
            .iter_mut()
            .filter(|r| r.id == room.id)
            .next()
            .ok_or(StoreError::NotExisting)?;
        existing_room.title = room.title;
        existing_room.description = room.description;
        existing_room.event_id = room.event_id;
        Ok(())
    }

    fn delete_room(
        &mut self,
        _auth_token: &AuthToken,
        _event_id: crate::data_store::EventId,
        room_id: crate::data_store::RoomId,
    ) -> Result<(), StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        data.rooms.retain(|r| r.id != room_id);
        Ok(())
    }

    fn get_categories(
        &mut self,
        _auth_token: &AuthToken,
        _event_id: crate::data_store::EventId,
    ) -> Result<Vec<Category>, StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        Ok(data.categories.clone())
    }

    fn create_category(
        &mut self,
        _auth_token: &AuthToken,
        category: NewCategory,
    ) -> Result<(), StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        data.categories.push(Category {
            id: category.id,
            title: category.title,
            icon: category.icon,
            color: category.color,
            event_id: category.event_id,
            is_official: category.is_official,
        });
        Ok(())
    }

    fn update_category(
        &mut self,
        _auth_token: &AuthToken,
        category: NewCategory,
    ) -> Result<(), StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        let existing_category = data
            .categories
            .iter_mut()
            .filter(|c| c.id == category.id)
            .next()
            .ok_or(StoreError::NotExisting)?;
        existing_category.title = category.title; 
        existing_category.icon = category.icon; 
        existing_category.color = category.color; 
        existing_category.event_id = category.event_id;
        existing_category.is_official = category.is_official;
        Ok(())
    }

    fn delete_category(
        &mut self,
        _auth_token: &AuthToken,
        _event_id: crate::data_store::EventId,
        category_id: crate::data_store::CategoryId,
    ) -> Result<(), StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        data.categories.retain(|c| c.id != category_id);
        Ok(())
    }

    fn authorize(
        &mut self,
        _event_id: i32,
        passphrase: &str,
        session_token: &mut SessionToken,
    ) -> Result<(), StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        if passphrase == "orga" {
            session_token.add_authorization(1);
            Ok(())
        } else if passphrase == "user" {
            session_token.add_authorization(2);
            Ok(())
        } else {
            Err(StoreError::NotExisting)
        }
    }

    fn check_authorization(
        &mut self,
        session_token: &SessionToken,
        event_id: EventId,
    ) -> Result<AuthToken, StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        if session_token.get_passphrase_ids().contains(&1) {
            Ok(AuthToken {
                event_id,
                roles: vec![AccessRole::Orga, AccessRole::User],
            })
        } else if session_token.get_passphrase_ids().contains(&2) {
            Ok(AuthToken {
                event_id,
                roles: vec![AccessRole::User],
            })
        } else {
            Err(StoreError::PermissionDenied)
        }
    }
}
