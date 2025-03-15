use crate::auth_session::SessionToken;
use crate::data_store::models::{
    Category, Event, FullEntry, FullNewEntry, NewCategory, NewEvent, NewRoom, Room,
};
use crate::data_store::{
    models, AccessRole, AuthToken, EntryFilter, EventId, GlobalAuthToken, KuaPlanStore,
    KueaPlanStoreFacade, StoreError,
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

    fn get_entries_filtered(
        &mut self,
        _auth_token: &AuthToken,
        _the_event_id: crate::data_store::EventId,
        filter: EntryFilter,
    ) -> Result<Vec<FullEntry>, StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        let mut result: Vec<FullEntry> = data
            .entries
            .iter()
            .filter(|e| filter.matches(e))
            .cloned()
            .collect();
        result.sort_by_key(|e| (e.entry.begin, e.entry.end));
        Ok(result)
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

    fn create_or_update_entry(
        &mut self,
        _auth_token: &AuthToken,
        entry: FullNewEntry,
    ) -> Result<bool, StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        let existing_entry = data
            .entries
            .iter_mut()
            .filter(|e| e.entry.id == entry.entry.id)
            .next();
        if let Some(e) = existing_entry {
            if (e.entry.event_id != entry.entry.event_id || e.entry.deleted) {
                return Err(StoreError::ConflictEntityExists);
            }
            e.entry.title = entry.entry.title;
            e.entry.description = entry.entry.description;
            e.entry.responsible_person = entry.entry.responsible_person;
            e.entry.is_room_reservation = entry.entry.is_room_reservation;
            e.entry.residue_of = entry.entry.residue_of;
            e.entry.event_id = entry.entry.event_id;
            e.entry.begin = entry.entry.begin;
            e.entry.end = entry.entry.end;
            e.entry.category = entry.entry.category;
            e.entry.last_updated = chrono::Utc::now();
            e.entry.comment = entry.entry.comment;
            e.entry.room_comment = entry.entry.room_comment;
            e.entry.time_comment = entry.entry.time_comment;
            e.entry.is_exclusive = entry.entry.is_exclusive;
            e.entry.is_cancelled = entry.entry.is_cancelled;
            e.room_ids = entry.room_ids;
            Ok(false)
        } else {
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
            Ok(true)
        }
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

    fn create_or_update_room(
        &mut self,
        _auth_token: &AuthToken,
        room: NewRoom,
    ) -> Result<bool, StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        let existing_room = data.rooms.iter_mut().filter(|r| r.id == room.id).next();
        if let Some(r) = existing_room {
            if r.event_id != room.event_id {
                return Err(StoreError::ConflictEntityExists);
            }
            r.title = room.title;
            r.description = room.description;
            Ok(false)
        } else {
            data.rooms.push(Room {
                id: room.id,
                title: room.title,
                description: room.description,
                event_id: room.event_id,
            });
            Ok(true)
        }
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

    fn create_or_update_category(
        &mut self,
        _auth_token: &AuthToken,
        category: NewCategory,
    ) -> Result<bool, StoreError> {
        let mut data = self.store.data.lock().expect("Error while locking mutex.");
        if let Some(e) = data.next_error.take() {
            return Err(e);
        }
        let existing_category = data
            .categories
            .iter_mut()
            .filter(|c| c.id == category.id)
            .next();
        if let Some(c) = existing_category {
            if c.event_id != category.event_id {
                return Err(StoreError::ConflictEntityExists);
            }
            c.title = category.title;
            c.icon = category.icon;
            c.color = category.color;
            c.event_id = category.event_id;
            c.is_official = category.is_official;
            Ok(false)
        } else {
            data.categories.push(Category {
                id: category.id,
                title: category.title,
                icon: category.icon,
                color: category.color,
                event_id: category.event_id,
                is_official: category.is_official,
            });
            Ok(true)
        }
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
