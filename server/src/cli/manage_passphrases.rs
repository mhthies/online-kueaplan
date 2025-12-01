use crate::cli::util::{query_user, query_user_bool};
use crate::cli::{CliAuthTokenKey, EventIdOrSlug};
use crate::cli_error::CliError;
use crate::data_store::auth_token::{AccessRole, AuthToken};
use crate::data_store::models::{Event, NewPassphrase, Passphrase, PassphrasePatch};
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
        .set_header(vec![
            "id",
            "role",
            "passphrase",
            "derivable from",
            "comment",
            "valid from",
            "valid until",
        ])
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
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
                passphrase.comment,
                passphrase
                    .valid_from
                    .map(|v| v.to_string())
                    .unwrap_or("∞".to_owned()),
                passphrase
                    .valid_until
                    .map(|v| v.to_string())
                    .unwrap_or("∞".to_owned()),
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
    let comment: String = query_user("Comment about designated usage");
    let valid_until = query_user::<IsoDateTime>(
        "Passphrase is valid until (YYYY-MM-DD hh:mm:ssZ; empty value for no limit)",
    )
    .0;

    let auth_key = CliAuthTokenKey::new();
    let auth_token = AuthToken::create_for_cli(event.id, &auth_key);
    let new_passphrase_id = data_store.create_passphrase(
        &auth_token,
        NewPassphrase {
            event_id: event.id,
            passphrase: Some(passphrase),
            privilege: access_role.0,
            derivable_from_passphrase: None,
            comment,
            valid_from: None,
            valid_until,
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
                comment: "".to_string(),
                valid_from: None,
                valid_until,
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

    print!("Deleting ");
    write_passphrase_id(std::io::stdout(), &event, passphrase).unwrap();
    println!();

    let confirm = query_user_bool(
        "Do you want to delete this passphrase and invalidate all sessions?",
        None,
    );
    if confirm {
        data_store.delete_passphrase(&auth_token, event.id, passphrase_id)?;
    }

    Ok(())
}

pub fn edit_passphrase(
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

    print!("Editing ");
    write_passphrase_id(std::io::stdout(), &event, passphrase).unwrap();
    println!();

    let edit_comment = query_user_bool("Edit the comment?", Some(false));
    let comment: String = if edit_comment {
        query_user("Comment about designated usage")
    } else {
        passphrase.comment.clone()
    };
    let edit_valid_from = query_user_bool("Edit validity begin?", Some(false));
    let valid_from = if edit_valid_from {
        query_user::<IsoDateTime>(
            "Passphrase is valid from (YYYY-MM-DD hh:mm:ssZ; empty value for no limit)",
        )
        .0
    } else {
        passphrase.valid_from
    };
    let edit_valid_until = query_user_bool("Edit validity end?", Some(false));
    let valid_until = if edit_valid_until {
        query_user::<IsoDateTime>(
            "Passphrase is valid until (YYYY-MM-DD hh:mm:ssZ; empty value for no limit)",
        )
        .0
    } else {
        passphrase.valid_until
    };

    data_store.patch_passphrase(
        &auth_token,
        passphrase_id,
        PassphrasePatch {
            comment: Some(comment),
            valid_from: Some(valid_from),
            valid_until: Some(valid_until),
        },
    )?;
    Ok(())
}

pub fn invalidate_passphrase(
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

    print!("Invalidating ");
    write_passphrase_id(std::io::stdout(), &event, passphrase).unwrap();
    println!();

    let confirm = query_user_bool(
        "Do you want to modify the passphrase to be invalid from now on?",
        None,
    );
    if confirm {
        data_store.patch_passphrase(
            &auth_token,
            passphrase_id,
            PassphrasePatch {
                valid_until: Some(Some(chrono::Utc::now())),
                ..Default::default()
            },
        )?;
    }

    Ok(())
}

#[allow(unused_must_use)]
fn write_passphrase_id(
    mut w: impl std::io::Write,
    event: &Event,
    passphrase: &Passphrase,
) -> std::io::Result<()> {
    if passphrase.passphrase.is_some() {
        write!(
            w,
            "passphrase '{}'",
            passphrase.passphrase.as_ref().unwrap().replace("\x7f", "*")
        )?;
    } else if passphrase.derivable_from_passphrase.is_some() {
        write!(w, "passphrase {}", passphrase.id)?;
    }
    if passphrase.derivable_from_passphrase.is_some() {
        write!(
            w,
            ", derivable from {}",
            passphrase.derivable_from_passphrase.unwrap()
        )?;
    }
    write!(w, ", for role {:?}", passphrase.privilege)?;
    if !passphrase.comment.is_empty() {
        write!(w, " ({})", passphrase.comment)?;
    }
    write!(w, " on {}", event.title)?;
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

struct IsoDateTime(Option<chrono::DateTime<chrono::Utc>>);

impl FromStr for IsoDateTime {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Self(None));
        }
        Ok(Self(Some(s.parse().map_err(|e| {
            format!("Could not parse as RFC3339 timestamp: {e}")
        })?)))
    }
}
