use crate::data_store::models::Event;
use crate::data_store::{EntryId, EventId};
use crate::web::ui::time_calculation;
use actix_web::error::UrlGenerationError;
use actix_web::HttpRequest;

/// Calculate the list of calendar days that the event covers
pub fn event_days(event: &Event) -> Vec<chrono::NaiveDate> {
    let len = (event.end_date - event.begin_date).num_days();
    (0..=len)
        .map(|i| event.begin_date + chrono::Duration::days(i))
        .collect()
}

/// Generate a URL that takes the user directly to a specific kueaplan entry in the main list.
///
/// The URL for the main_list endpoint with the correct date, according to the entry's begin is
/// used, augmented with the anchor link of the entry,
pub fn url_for_entry(
    req: &HttpRequest,
    event_id: EventId,
    entry_id: &EntryId,
    entry_begin: &chrono::DateTime<chrono::Utc>,
) -> Result<url::Url, UrlGenerationError> {
    let mut url = req.url_for(
        "main_list",
        [
            &event_id.to_string(),
            &time_calculation::get_effective_date(entry_begin).to_string(),
        ],
    )?;
    url.set_fragment(Some(&format!("entry-{}", entry_id)));
    Ok(url)
}
