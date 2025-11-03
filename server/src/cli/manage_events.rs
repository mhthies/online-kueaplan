use crate::cli_error::CliError;
use crate::data_store::get_store_from_env;
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
