use crate::cli::{CliAuthTokenKey, EventIdOrSlug};
use crate::cli_error::CliError;
use crate::data_store::auth_token::AuthToken;
use crate::data_store::get_store_from_env;
use crate::data_store::KuaPlanStore;

pub fn print_passphrase_list(event_id_or_slug: EventIdOrSlug) -> Result<(), CliError> {
    let data_store_pool = get_store_from_env()?;
    let mut data_store = data_store_pool.get_facade()?;

    let event = match event_id_or_slug {
        EventIdOrSlug::Id(event_id) => data_store.get_event(event_id)?,
        EventIdOrSlug::Slug(event_slug) => data_store.get_event_by_slug(&event_slug)?,
    };

    let auth_key = CliAuthTokenKey::new();
    let auth_token = AuthToken::create_for_cli(event.id, &auth_key);
    let passphrases = data_store.get_passphrases(&auth_token, event.id)?;

    let mut table = comfy_table::Table::new();
    table
        .load_preset(comfy_table::presets::ASCII_BORDERS_ONLY_CONDENSED)
        .set_header(vec!["id", "role", "passphrase", "derivable from"])
        .add_rows(passphrases.into_iter().map(|passphrase| {
            [
                passphrase.id.to_string(),
                format!("{:?}", passphrase.privilege),
                passphrase
                    .passphrase
                    .unwrap_or("".to_string())
                    .replace("\x7f", "*"),
                passphrase
                    .derivable_from_passphrase
                    .map(|pid| pid.to_string())
                    .unwrap_or("".to_string()),
            ]
        }));

    println!("Passphrases of event {}:", event.title);
    println!("{table}");
    Ok(())
}
