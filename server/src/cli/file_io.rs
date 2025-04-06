use crate::cli::CliAuthTokenKey;
use crate::data_store::auth_token::{AuthToken, GlobalAuthToken};
use crate::data_store::models::{FullNewEntry, NewCategory, NewRoom};
use crate::data_store::{get_store_from_env, KuaPlanStore};
use kueaplan_api_types::{Category, Entry, Event, Room};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct SavedEvent {
    event: Event,
    entries: Vec<Entry>,
    rooms: Vec<Room>,
    categories: Vec<Category>,
}

pub fn load_event_from_file(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // TODO logging instead of propagating error
    let data_store_pool = get_store_from_env()?;
    let mut data_store = data_store_pool.get_facade()?;

    let f = File::open(path)?;
    let data: SavedEvent = serde_json::from_reader(BufReader::new(f))?;

    let auth_key = CliAuthTokenKey::new();
    let admin_auth_token = GlobalAuthToken::create_for_cli(&auth_key);
    let event_id = data_store.create_event(&admin_auth_token, data.event.into())?;

    let auth_token = AuthToken::create_for_cli(event_id, &auth_key);
    for room in data.rooms {
        data_store.create_or_update_room(&auth_token, NewRoom::from_api(room, event_id))?;
    }
    for category in data.categories {
        data_store
            .create_or_update_category(&auth_token, NewCategory::from_api(category, event_id))?;
    }
    for entry in data.entries {
        data_store.create_or_update_entry(
            &auth_token,
            FullNewEntry::from_api(entry, event_id),
            false,
        )?;
    }

    Ok(())
}
