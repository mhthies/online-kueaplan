use crate::cli::util::{query_user, query_user_bool};
use crate::cli::{CliAuthTokenKey, EventIdOrSlug};
use crate::cli_error::CliError;
use crate::data_store::auth_token::{AccessRole, AuthToken};
use crate::data_store::models::NewPassphrase;
use crate::data_store::KuaPlanStore;
use crate::data_store::{get_store_from_env, PassphraseId};
use std::str::FromStr;

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

pub fn add_passphrase(event_id_or_slug: EventIdOrSlug) -> Result<(), CliError> {
    let data_store_pool = get_store_from_env()?;
    let mut data_store = data_store_pool.get_facade()?;

    let event = match event_id_or_slug {
        EventIdOrSlug::Id(event_id) => data_store.get_event(event_id)?,
        EventIdOrSlug::Slug(event_slug) => data_store.get_event_by_slug(&event_slug)?,
    };
    println!("Creating passphrase for {}", event.title);

    let access_role: PassphraseAccessRoleEntry = query_user("Enter access role");
    let passphrase: String = query_user("Enter passphrase");

    let auth_key = CliAuthTokenKey::new();
    let auth_token = AuthToken::create_for_cli(event.id, &auth_key);
    let new_passphrase_id = data_store.create_passphrase(
        &auth_token,
        NewPassphrase {
            event_id: event.id,
            passphrase: Some(passphrase),
            privilege: access_role.0,
            derivable_from_passphrase: None,
        },
    )?;
    println!("Success. New passphrase id: {}", new_passphrase_id);

    let create_derivable_passphrase =
        query_user_bool("Create derivable passphrase for link-sharing?", Some(true));
    if create_derivable_passphrase {
        data_store.create_passphrase(
            &auth_token,
            NewPassphrase {
                event_id: event.id,
                passphrase: None,
                privilege: AccessRole::SharableViewLink,
                derivable_from_passphrase: Some(new_passphrase_id),
            },
        )?;
        println!("Success.");
    }
    Ok(())
}

pub fn delete_passphrase(
    event_id_or_slug: EventIdOrSlug,
    passphrase_id: PassphraseId,
) -> Result<(), CliError> {
    let data_store_pool = get_store_from_env()?;
    let mut data_store = data_store_pool.get_facade()?;

    let event = match event_id_or_slug {
        EventIdOrSlug::Id(event_id) => data_store.get_event(event_id)?,
        EventIdOrSlug::Slug(event_slug) => data_store.get_event_by_slug(&event_slug)?,
    };
    let auth_key = CliAuthTokenKey::new();
    let auth_token = AuthToken::create_for_cli(event.id, &auth_key);
    let passphrases = data_store.get_passphrases(&auth_token, event.id)?;
    let passphrase =
        passphrases
            .iter()
            .find(|p| p.id == passphrase_id)
            .ok_or(CliError::DataError(
                "Passphrase with given id does not exist".to_string(),
            ))?;

    if passphrase.passphrase.is_some() {
        println!(
            "Deleting passphrase '{}' for role {:?} on {}",
            passphrase.passphrase.as_ref().unwrap().replace("\x7f", "*"),
            passphrase.privilege,
            event.title
        );
    } else if passphrase.derivable_from_passphrase.is_some() {
        println!(
            "Deleting passphrase {}, derivable from {}, for role {:?} on {}",
            passphrase.id,
            passphrase.derivable_from_passphrase.unwrap(),
            passphrase.privilege,
            event.title
        );
    } else {
        println!(
            "Deleting passphrase {}, for role {:?} on {}",
            passphrase.id, passphrase.privilege, event.title
        );
    }

    let confirm = query_user_bool(
        "Do you want to delete this passphrase and invalidate all sessions?",
        None,
    );
    if confirm {
        data_store.delete_passphrase(&auth_token, event.id, passphrase_id)?;
    }

    Ok(())
}

struct PassphraseAccessRoleEntry(AccessRole);

impl FromStr for PassphraseAccessRoleEntry {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "u" | "user" => Ok(Self(AccessRole::User)),
            "o" | "orga" => Ok(Self(AccessRole::Orga)),
            "a" | "admin" => Ok(Self(AccessRole::Admin)),
            _ => Err("Unknown access role. Must be 'user', 'orga' or 'admin'."),
        }
    }
}
