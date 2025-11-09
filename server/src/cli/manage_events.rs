use crate::cli::util::{query_user, query_user_and_check, query_user_bool};
use crate::cli::{CliAuthTokenKey, EventIdOrSlug};
use crate::cli_error::CliError;
use crate::data_store::auth_token::{AccessRole, AuthToken, GlobalAuthToken};
use crate::data_store::get_store_from_env;
use crate::data_store::models::{
    Event, EventClockInfo, EventDayScheduleSection, EventDayTimeSchedule, ExtendedEvent,
    NewCategory, NewPassphrase,
};
use crate::data_store::{EventFilter, KuaPlanStore};
use uuid::Uuid;

pub fn print_event_list() -> Result<(), CliError> {
    let data_store_pool = get_store_from_env()?;
    let mut data_store = data_store_pool.get_facade()?;

    let events = data_store.get_events(EventFilter::default())?;

    let mut table = comfy_table::Table::new();
    table
        .load_preset(comfy_table::presets::ASCII_BORDERS_ONLY_CONDENSED)
        .set_header(vec!["id", "slug", "title", "begin", "end"])
        .add_rows(events.into_iter().map(|event| {
            [
                event.id.to_string(),
                event.slug.unwrap_or(String::new()),
                event.title,
                event.begin_date.to_string(),
                event.end_date.to_string(),
            ]
        }));

    println!("{table}");
    Ok(())
}

pub fn create_event() -> Result<(), CliError> {
    let data_store_pool = get_store_from_env()?;
    let mut data_store = data_store_pool.get_facade()?;
    let auth_key = CliAuthTokenKey::new();
    let auth = GlobalAuthToken::create_for_cli(&auth_key);

    let title: String =
        query_user_and_check("Enter event title (e.g. Pfingsten25)", |title: &String| {
            if title.is_empty() {
                Err("event title must not be empty")
            } else {
                Ok(())
            }
        });
    let slug: String = query_user("Enter event slug (e.g. pa25)");
    let begin_date: chrono::NaiveDate = query_user("Enter event begin (YYYY-MM-DD)");
    let end_date: chrono::NaiveDate = query_user("Enter event end (YYYY-MM-DD)");

    let event = ExtendedEvent {
        basic_data: Event {
            id: 0,
            title: title.clone(),
            begin_date,
            end_date,
            slug: (!slug.is_empty()).then(|| slug),
        },
        clock_info: EventClockInfo {
            timezone: chrono_tz::Tz::Europe__Berlin,
            effective_begin_of_day: chrono::NaiveTime::from_hms_opt(5, 30, 0).unwrap(),
        },
        default_time_schedule: EventDayTimeSchedule {
            sections: vec![
                EventDayScheduleSection {
                    name: "vom Vortag".to_owned(),
                    end_time: Some(chrono::NaiveTime::from_hms_opt(5, 30, 0).unwrap()),
                },
                EventDayScheduleSection {
                    name: "Morgens".to_owned(),
                    end_time: Some(chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap()),
                },
                EventDayScheduleSection {
                    name: "Mittags".to_owned(),
                    end_time: Some(chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap()),
                },
                EventDayScheduleSection {
                    name: "Abends".to_owned(),
                    end_time: None,
                },
            ],
        },
        preceding_event_id: None,
        subsequent_event_id: None,
    };

    let event_id = data_store.create_event(&auth, event)?;
    let auth_token = AuthToken::create_for_cli(event_id, &auth_key);
    println!("\nNew event '{}' created with id {}\n", title, event_id);
    data_store.create_or_update_category(
        &auth_token,
        NewCategory {
            id: Uuid::now_v7(),
            title: "Keine Kategorie".to_string(),
            icon: "".to_string(),
            color: "99aabb".to_string(),
            event_id,
            is_official: false,
            sort_key: 0,
        },
    )?;

    let create_passphrase = query_user_bool("Create admin passphrase?", Some(true));
    if create_passphrase {
        let passphrase: String = query_user("Enter admin passphrase");
        data_store.create_passphrase(
            &auth_token,
            NewPassphrase {
                event_id,
                privilege: AccessRole::Admin,
                passphrase: Some(passphrase),
                derivable_from_passphrase: None,
            },
        )?;
        println!("New passphrase has been created successfully.");
    }

    Ok(())
}

pub fn delete_event(event_id_or_slug: EventIdOrSlug) -> Result<(), CliError> {
    let data_store_pool = get_store_from_env()?;
    let mut data_store = data_store_pool.get_facade()?;
    let event = match event_id_or_slug {
        EventIdOrSlug::Id(event_id) => data_store.get_event(event_id)?,
        EventIdOrSlug::Slug(event_slug) => data_store.get_event_by_slug(&event_slug)?,
    };

    let auth_key = CliAuthTokenKey::new();
    let auth_token = AuthToken::create_for_cli(event.id, &auth_key);

    println!("The event '{}' (id={}) will be deleted with all its associated data (entries, categories, rooms, announcements).", event.title, event.id);
    println!("Are you sure to irreversibly delete the event and all its data?");
    query_user_and_check::<String, _, _>("To confirm deletion, enter the event's title", |input| {
        if *input == event.title {
            Ok(())
        } else {
            Err("Title not entered correctly")
        }
    });
    data_store.delete_event(&auth_token, event.id)?;

    println!("Success");
    Ok(())
}
