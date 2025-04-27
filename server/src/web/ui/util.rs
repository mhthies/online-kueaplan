use crate::auth_session::SessionToken;
use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Event, FullEntry};
use crate::data_store::{EntryId, EventId};
use crate::web::ui::error::AppError;
use crate::web::ui::time_calculation;
use crate::web::AppState;
use actix_web::error::UrlGenerationError;
use actix_web::HttpRequest;

#[allow(clippy::identity_op)] // We want to explicitly state that it's "1" year
pub const SESSION_COOKIE_MAX_AGE: std::time::Duration =
    std::time::Duration::from_secs(1 * 86400 * 365);
pub const SESSION_COOKIE_NAME: &str = "kuea-plan-session";

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

/// Generate a URL that takes the user to the main list for the given event day.
pub fn url_for_main_list(
    req: &HttpRequest,
    event_id: EventId,
    date: &chrono::NaiveDate,
) -> Result<String, UrlGenerationError> {
    Ok(req
        .url_for("main_list", &[event_id.to_string(), date.to_string()])?
        .to_string())
}

/// Generate a URL for editing the given KüA-Plan entry
pub fn url_for_edit_entry(
    req: &HttpRequest,
    entry: &FullEntry,
) -> Result<String, UrlGenerationError> {
    Ok(req
        .url_for(
            "edit_entry_form",
            &[entry.entry.event_id.to_string(), entry.entry.id.to_string()],
        )?
        .to_string())
}

/// Extract the session token from the session token cookie and validate it
///
/// The `privilege` and `event_id` parameters are not validated here, but only used for better error
/// reporting.
pub fn extract_session_token(
    app_state: &AppState,
    request: &HttpRequest,
    for_privilege: Privilege,
    for_event_id: EventId,
) -> Result<SessionToken, AppError> {
    SessionToken::from_string(
        request
            .cookie(SESSION_COOKIE_NAME)
            .ok_or(AppError::PermissionDenied {
                required_privilege: for_privilege,
                event_id: for_event_id,
                session_error: None,
            })?
            .value(),
        &app_state.secret,
        SESSION_COOKIE_MAX_AGE,
    )
    .map_err(|session_error| AppError::PermissionDenied {
        required_privilege: for_privilege,
        event_id: for_event_id,
        session_error: Some(session_error),
    })
}

pub fn privilege_access_roles_names(privilege: &Privilege) -> Vec<&'static str> {
    privilege
        .qualifying_roles()
        .iter()
        .map(|r| r.name())
        .collect()
}
