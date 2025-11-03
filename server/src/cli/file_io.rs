use crate::cli::{CliAuthTokenKey, EventIdOrSlug};
use crate::cli_error::CliError;
use crate::data_store::auth_token::{AuthToken, GlobalAuthToken};
use crate::data_store::models;
use crate::data_store::{get_store_from_env, EntryFilter, EventId, KuaPlanStore};
use kueaplan_api_types::{Announcement, Category, Entry, ExtendedEvent, Room};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct SavedEvent {
    event: ExtendedEvent,
    entries: Vec<Entry>,
    rooms: Vec<Room>,
    categories: Vec<Category>,
    #[serde(default)]
    announcements: Vec<Announcement>,
}

pub fn load_event_from_file(path: &PathBuf) -> Result<(), CliError> {
    // TODO logging instead of propagating error
    let data_store_pool = get_store_from_env()?;
    let mut data_store = data_store_pool.get_facade()?;

    let f = File::open(path).map_err(|e| {
        CliError::FileError(format!("Could not open {:?} for reading: {}", path, e))
    })?;
    let data: SavedEvent = serde_json::from_reader(BufReader::new(f))?;

    let auth_key = CliAuthTokenKey::new();
    let admin_auth_token = GlobalAuthToken::create_for_cli(&auth_key);
    let event_id = data_store.create_event(
        &admin_auth_token,
        data.event.try_into().map_err(|e| CliError::DataError(e))?,
    )?;

    let auth_token = AuthToken::create_for_cli(event_id, &auth_key);
    for room in data.rooms {
        data_store.create_or_update_room(&auth_token, models::NewRoom::from_api(room, event_id))?;
    }
    for category in data.categories {
        data_store.create_or_update_category(
            &auth_token,
            models::NewCategory::from_api(category, event_id),
        )?;
    }
    for entry in data.entries {
        data_store.create_or_update_entry(
            &auth_token,
            models::FullNewEntry::from_api(entry, event_id),
            false,
            None,
        )?;
    }
    for announcement in data.announcements {
        data_store.create_or_update_announcement(
            &auth_token,
            models::FullNewAnnouncement::from_api(announcement, event_id),
            None,
        )?;
    }

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
