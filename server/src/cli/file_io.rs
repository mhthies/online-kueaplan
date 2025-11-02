use crate::cli::CliAuthTokenKey;
use crate::cli_error::CliError;
use crate::data_store::auth_token::{AuthToken, GlobalAuthToken};
use crate::data_store::models;
use crate::data_store::{get_store_from_env, KuaPlanStore};
use kueaplan_api_types::{Announcement, Category, Entry, ExtendedEvent, Room};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
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
