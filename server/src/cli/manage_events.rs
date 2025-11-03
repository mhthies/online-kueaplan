use crate::cli::util::{query_user, query_user_and_check};
use crate::cli::CliAuthTokenKey;
use crate::cli_error::CliError;
use crate::data_store::auth_token::GlobalAuthToken;
use crate::data_store::get_store_from_env;
use crate::data_store::models::{
    Event, EventClockInfo, EventDayScheduleSection, EventDayTimeSchedule, ExtendedEvent,
};
use crate::data_store::{EventFilter, KuaPlanStore};

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
    println!("New event '{}' created with id {}", title, event_id);

    Ok(())
}
