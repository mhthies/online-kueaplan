use crate::cli::{CliAuthTokenKey, EventIdOrSlug};
use crate::cli_error::CliError;
use crate::data_store::auth_token::{AuthToken, GlobalAuthToken};
use crate::data_store::models::EventWithContents;
use crate::data_store::{get_store_from_env, EntryFilter, KuaPlanStore};
use crate::data_store::{models, CategoryId, RoomId};
use kueaplan_api_types::{Announcement, Category, Entry, ExtendedEvent, Room};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct SavedEvent {
    event: ExtendedEvent,
    entries: Vec<Entry>,
    rooms: Vec<Room>,
    categories: Vec<Category>,
    #[serde(default)]
    announcements: Vec<Announcement>,
}

pub fn load_event_from_file(path: &PathBuf, generate_new_uuids: bool) -> Result<(), CliError> {
    let data_store_pool = get_store_from_env()?;
    let mut data_store = data_store_pool.get_facade()?;

    let f = File::open(path).map_err(|e| {
        CliError::FileError(format!("Could not open {:?} for reading: {}", path, e))
    })?;
    let mut data: SavedEvent = serde_json::from_reader(BufReader::new(f))?;

    if generate_new_uuids {
        regenerate_uuids(&mut data)?;
    }
    let data = data;

    let auth_key = CliAuthTokenKey::new();
    let admin_auth_token = GlobalAuthToken::create_for_cli(&auth_key);
    let store_data = EventWithContents {
        event: data.event.try_into().map_err(|e| CliError::DataError(e))?,
        rooms: data
            .rooms
            .into_iter()
            .map(|r| models::NewRoom::from_api(r, -1))
            .collect(),
        categories: data
            .categories
            .into_iter()
            .map(|c| models::NewCategory::from_api(c, -1))
            .collect(),
        entries: data
            .entries
            .into_iter()
            .map(|e| models::FullNewEntry::from_api(e, -1))
            .collect(),
        announcements: data
            .announcements
            .into_iter()
            .map(|a| models::FullNewAnnouncement::from_api(a, -1))
            .collect(),
    };

    data_store.import_event_with_contents(&admin_auth_token, store_data)?;

    Ok(())
}

pub fn export_event_to_file(
    event_id_or_slug: EventIdOrSlug,
    path: &PathBuf,
) -> Result<(), CliError> {
    let data_store_pool = get_store_from_env()?;
    let mut data_store = data_store_pool.get_facade()?;

    let event_id = match event_id_or_slug {
        EventIdOrSlug::Id(event_id) => event_id,
        EventIdOrSlug::Slug(event_slug) => {
            let basic_event = data_store.get_event_by_slug(&event_slug)?;
            basic_event.id
        }
    };

    let auth_key = CliAuthTokenKey::new();
    let auth_token = AuthToken::create_for_cli(event_id, &auth_key);

    let data = SavedEvent {
        event: data_store.get_extended_event(&auth_token, event_id)?.into(),
        entries: data_store
            .get_entries_filtered(&auth_token, event_id, EntryFilter::default())?
            .into_iter()
            .map(|e| e.into())
            .collect(),
        rooms: data_store
            .get_rooms(&auth_token, event_id)?
            .into_iter()
            .map(|r| r.into())
            .collect(),
        categories: data_store
            .get_categories(&auth_token, event_id)?
            .into_iter()
            .map(|c| c.into())
            .collect(),
        announcements: data_store
            .get_announcements(&auth_token, event_id, None)?
            .into_iter()
            .map(|a| a.into())
            .collect(),
    };

    let f = File::create(path).map_err(|e| {
        CliError::FileError(format!(
            "Could not create or open {:?} for writing: {}",
            path, e
        ))
    })?;
    serde_json::to_writer(BufWriter::new(f), &data)?;

    Ok(())
}

fn regenerate_uuids(event_data: &mut SavedEvent) -> Result<(), CliError> {
    let mut room_id_map = BTreeMap::<RoomId, RoomId>::new();
    for room in event_data.rooms.iter_mut() {
        let new_id = Uuid::now_v7();
        room_id_map.insert(room.id, new_id);
        room.id = new_id;
    }
    let mut category_id_map = BTreeMap::<CategoryId, CategoryId>::new();
    for category in event_data.categories.iter_mut() {
        let new_id = Uuid::now_v7();
        category_id_map.insert(category.id, new_id);
        category.id = new_id;
    }
    for entry in event_data.entries.iter_mut() {
        entry.category = *category_id_map
            .get(&entry.category)
            .ok_or(CliError::DataError(format!(
                "Category {} of entry {} does not exist",
                entry.category, entry.id
            )))?;
        for entry_room in entry.room.iter_mut() {
            *entry_room = *room_id_map
                .get(&entry_room)
                .ok_or(CliError::DataError(format!(
                    "Room {} of entry {} does not exist",
                    entry_room, entry.id
                )))?;
        }
        for previous_date in entry.previous_dates.iter_mut() {
            for previous_date_room in previous_date.room.iter_mut() {
                *previous_date_room =
                    *room_id_map
                        .get(&previous_date_room)
                        .ok_or(CliError::DataError(format!(
                            "Room {} of previous date {} of entry {} does not exist",
                            previous_date_room, previous_date.id, entry.id
                        )))?;
            }
            previous_date.id = Uuid::now_v7();
        }
        entry.id = Uuid::now_v7();
    }
    for announcement in event_data.announcements.iter_mut() {
        for announcement_category in announcement.categories.iter_mut() {
            *announcement_category =
                *category_id_map
                    .get(&announcement_category)
                    .ok_or(CliError::DataError(format!(
                        "Category {} of announcement {} does not exist",
                        announcement_category, announcement.id
                    )))?;
        }
        for announcement_room in announcement.rooms.iter_mut() {
            *announcement_room =
                *room_id_map
                    .get(&announcement_room)
                    .ok_or(CliError::DataError(format!(
                        "Room {} of announcement {} does not exist",
                        announcement_room, announcement.id
                    )))?;
        }
        announcement.id = Uuid::now_v7();
    }

    Ok(())
}
