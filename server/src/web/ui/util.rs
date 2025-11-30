use crate::auth_session::SessionToken;
use crate::data_store::auth_token::{AccessRole, Privilege};
use crate::data_store::models::{AnnouncementType, Event, EventClockInfo, FullEntry};
use crate::data_store::{EntryId, EventId, StoreError};
use crate::web::time_calculation::get_effective_date;
use crate::web::ui::error::AppError;
use crate::web::ui::flash::{FlashMessage, FlashMessageActionButton, FlashType, FlashesInterface};
use crate::web::ui::form_values::{FormValue, _FormValidSimpleValidate};
use crate::web::ui::sub_templates::main_list_row::MainListRow;
use crate::web::ui::validation;
use crate::web::AppState;
use actix_web::error::UrlGenerationError;
use actix_web::web::Redirect;
use actix_web::{Either, HttpRequest, HttpResponse};
use askama::Template;
use chrono::Weekday;
use chrono::{Datelike, TimeZone};

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
pub fn url_for_entry_details(
    req: &HttpRequest,
    event_id: EventId,
    entry_id: &EntryId,
    entry_begin_effective_date: &chrono::NaiveDate,
) -> Result<url::Url, UrlGenerationError> {
    let mut url = req.url_for(
        "main_list",
        [
            &event_id.to_string(),
            &entry_begin_effective_date.to_string(),
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
                privilege_expired: false,
            })?
            .value(),
        &app_state.secret,
        SESSION_COOKIE_MAX_AGE,
    )
    .map_err(|session_error| AppError::PermissionDenied {
        required_privilege: for_privilege,
        event_id: for_event_id,
        session_error: Some(session_error),
        privilege_expired: false,
    })
}

/// Extract the session token from the session token cookie and validate it, if it exists
///
/// In contrast to [extract_session_token], this function does not produce an `Err` result, when no
/// session token is present, but an `Ok(None)` value. However, other error conditions (like invalid
/// or timed-out session tokens) are still reported as an error.
pub fn extract_session_token_if_present(
    app_state: &AppState,
    request: &HttpRequest,
    for_privilege: Privilege,
    for_event_id: EventId,
) -> Result<Option<SessionToken>, AppError> {
    match extract_session_token(app_state, request, for_privilege, for_event_id) {
        Ok(token) => Ok(Some(token)),
        Err(AppError::PermissionDenied {
            session_error: None,
            ..
        }) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn privilege_access_roles_names(privilege: &Privilege) -> Vec<&'static str> {
    privilege
        .qualifying_roles()
        .iter()
        .map(|r| r.name())
        .collect()
}

/// Convert a date to the (german) name of its weekday
pub fn weekday(date: &chrono::NaiveDate) -> &'static str {
    match date.weekday() {
        Weekday::Mon => "Montag",
        Weekday::Tue => "Dienstag",
        Weekday::Wed => "Mittwoch",
        Weekday::Thu => "Donnerstag",
        Weekday::Fri => "Freitag",
        Weekday::Sat => "Samstag",
        Weekday::Sun => "Sonntag",
    }
}

/// Convert a date to a short version of the (german) name of its weekday
pub fn weekday_short(date: &chrono::NaiveDate) -> &'static str {
    match date.weekday() {
        Weekday::Mon => "Mo",
        Weekday::Tue => "Di",
        Weekday::Wed => "Mi",
        Weekday::Thu => "Do",
        Weekday::Fri => "Fr",
        Weekday::Sat => "Sa",
        Weekday::Sun => "So",
    }
}

/// Selects the Bootstrap Icons icon name for the given [AnnouncementType]
pub fn announcement_type_icon(announcement_type: AnnouncementType) -> &'static str {
    match announcement_type {
        AnnouncementType::Info => "info-circle-fill",
        AnnouncementType::Warning => "exclamation-triangle",
    }
}

/// Returns the name of the given [AnnouncementType]
pub fn announcement_type_name(announcement_type: AnnouncementType) -> &'static str {
    match announcement_type {
        AnnouncementType::Info => "Information",
        AnnouncementType::Warning => "Warnung",
    }
}
/// Returns the Bootstrap color name for the given [AnnouncementType]
pub fn announcement_type_color(announcement_type: AnnouncementType) -> &'static str {
    match announcement_type {
        AnnouncementType::Info => "info",
        AnnouncementType::Warning => "warning",
    }
}

/// Helper type for representing the different possible outcomes of submitting the edit form.
///
/// They are used to delegate creating appropriate response to [create_edit_form_response()].
pub enum FormSubmitResult {
    Success,
    ValidationError,
    TransactionConflict,
    ConcurrentEditConflict,
    UnexpectedError(AppError),
}

impl From<Result<(), StoreError>> for FormSubmitResult {
    fn from(value: Result<(), StoreError>) -> Self {
        match value {
            Ok(()) => FormSubmitResult::Success,
            Err(e) => match e {
                StoreError::TransactionConflict => FormSubmitResult::TransactionConflict,
                StoreError::ConcurrentEditConflict => FormSubmitResult::ConcurrentEditConflict,
                _ => FormSubmitResult::UnexpectedError(e.into()),
            },
        }
    }
}

/// Helper function for generating the HTTP response in [edit_entry()].
///
/// Together with the [FormSubmitResult] helper type, this function helps keeping the code of
/// edit_entry() and similar functions more readable. Without these tricks we'd have error message
/// creation functions scattered all over the code.
///
/// Depending on the `result`, the function either creates
/// * a redirect to the `success_redirect` with an appropriate flash message, in case of
///   [FormSubmitResult::Success]
/// * the rendered `form_template` with a flash message linking to `form_url`, in case of
///   [FormSubmitResult::ConcurrentEditConflict]
/// * the rendered `form_template` with a flash message allowing to resubmit the form `form_name`,
///   in case of [FormSubmitResult::TransactionConflict]
/// * the rendered `form_template` with an appropriate flash message in case of
///   [FormSubmitResult::ValidationError]
/// * an [AppError] in case of any [FormSubmitResult::UnexpectedError]
#[allow(clippy::too_many_arguments)]
pub fn create_edit_form_response(
    result: FormSubmitResult,
    form_template: impl Template,
    name_of_thing: &'static str,
    form_url: url::Url,
    form_name: &'static str,
    is_new_entity: bool,
    success_redirect: url::Url,
    request: &HttpRequest,
) -> Result<Either<Redirect, HttpResponse>, AppError> {
    match result {
        FormSubmitResult::Success => {
            request.add_flash_message(FlashMessage {
                flash_type: FlashType::Success,
                message: if is_new_entity {
                    format!("{} wurde gespeichert.", name_of_thing)
                } else {
                    "Änderung wurde gespeichert.".to_owned()
                },
                keep_open: false,
                button: None,
            });
            Ok(Either::Left(
                Redirect::to(success_redirect.to_string()).see_other(),
            ))
        }
        FormSubmitResult::ValidationError => {
            request.add_flash_message(FlashMessage {
                flash_type: FlashType::Error,
                message: "Eingegebene Daten sind ungültig. Bitte markierte Felder überprüfen."
                    .to_owned(),
                keep_open: false,
                button: None,
            });
            Ok(Either::Right(
                HttpResponse::UnprocessableEntity().body(form_template.render()?),
            ))
        }
        FormSubmitResult::ConcurrentEditConflict => {
            request.add_flash_message(FlashMessage {
                flash_type: FlashType::Error,
                message: format!("{} wurde zwischenzeitlich bearbeitet. Bitte das Formular neu laden und die Änderung erneut durchführen.", name_of_thing),
                keep_open: true,
                button: Some(FlashMessageActionButton::ReloadCleanForm {
                    form_url: form_url.to_string(),
                }),
            });
            Ok(Either::Right(
                HttpResponse::Conflict().body(form_template.render()?),
            ))
        }
        FormSubmitResult::TransactionConflict => {
            request.add_flash_message(FlashMessage {
                flash_type: FlashType::Warning,
                message: "Konnte wegen parallelem Datenbank-Zugriff nicht speichern. Bitte Formular erneut absenden."
                    .to_owned(),
                keep_open: true,
                button: Some(FlashMessageActionButton::SubmitForm { form_id: form_name.to_string() }),
            });
            Ok(Either::Right(
                HttpResponse::ServiceUnavailable().body(form_template.render()?),
            ))
        }
        FormSubmitResult::UnexpectedError(e) => Err(e),
    }
}

/// Generate the list of [MainListRow]s from the given list of KüA-Plan `entries`.
///
/// This algorithm creates a MainListEntry for each entry and each previous_date of an entry,
/// sorts them by `begin` and merges consecutive list rows.
/// This is a simplified version of [main_list::generate_filtered_merged_list_entries].
pub fn generate_merged_list_rows_per_date<'a>(
    entries: &'a [FullEntry],
    clock_info: &'_ EventClockInfo,
) -> Vec<MainListRow<'a>> {
    let mut result = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        result.push(MainListRow::from_entry(entry));
        for previous_date in entry.previous_dates.iter() {
            result.push(MainListRow::from_previous_date(entry, previous_date))
        }
    }
    result.sort_by_key(|e| e.sort_time);
    result.dedup_by(|a, b| {
        if a.entry.entry.id == b.entry.entry.id
            && get_effective_date(a.sort_time, clock_info)
                == get_effective_date(b.sort_time, clock_info)
        {
            b.merge_from(a);
            true
        } else {
            false
        }
    });
    result
}

/// Group the rows of the main list into blocks by effective date.
///
/// The list must be already be sorted by [MainListRow::sort_time].
pub fn group_rows_by_date<'a>(
    entries: &'a Vec<MainListRow<'a>>,
    clock_info: &'_ EventClockInfo,
) -> Vec<(chrono::NaiveDate, Vec<&'a MainListRow<'a>>)> {
    let mut result = Vec::new();
    let mut block_entries = Vec::new();
    if entries.is_empty() {
        return result;
    }
    let mut current_date = get_effective_date(entries[0].sort_time, clock_info);
    for entry in entries {
        if get_effective_date(entry.sort_time, clock_info) != current_date {
            if !block_entries.is_empty() {
                result.push((current_date, block_entries));
            }
            block_entries = Vec::new();
            current_date = get_effective_date(entry.sort_time, clock_info);
        }
        block_entries.push(entry);
    }
    if !block_entries.is_empty() {
        result.push((current_date, block_entries));
    }
    result
}

/// Set the `is_first_row_of_next_calendar_date` flag on the first row in the list which starts
/// after the given context date (in calendar days)
pub fn mark_first_row_of_next_calendar_date(
    rows: &mut Vec<MainListRow>,
    date_context: chrono::NaiveDate,
    timezone: &chrono_tz::Tz,
) {
    for row in rows.iter_mut() {
        if row.sort_time.with_timezone(timezone).date_naive() > date_context {
            row.is_first_row_of_next_calendar_date = true;
            break;
        }
    }
}

/// Set the `is_first_row_of_next_calendar_date` flag on the first rows which starts
/// on the calendar day after its effective date, for each effective date.
///
/// The list must be already be sorted by [MainListRow::sort_time].
pub fn mark_first_row_of_next_calendar_date_per_effective_date(
    rows: &mut Vec<MainListRow>,
    clock_info: &EventClockInfo,
) {
    let mut current_effective_date = None;
    let mut found_first_row_of_current_date = false;
    for row in rows.iter_mut() {
        if Some(get_effective_date(row.sort_time, clock_info)) != current_effective_date {
            found_first_row_of_current_date = false;
            current_effective_date = Some(get_effective_date(row.sort_time, clock_info));
        }
        if row
            .sort_time
            .with_timezone(&clock_info.timezone)
            .date_naive()
            > get_effective_date(row.sort_time, clock_info)
            && !found_first_row_of_current_date
        {
            row.is_first_row_of_next_calendar_date = true;
            found_first_row_of_current_date = true;
        }
    }
}

/// Format an obfuscated passphrase (see [crate::data_store::postgres::obfuscate_passphrase]) for
/// displaying on Passphrase management sites.
pub fn format_passphrase(passphrase: &Option<String>) -> String {
    passphrase.as_deref().unwrap_or("").replace("\x7f", "*")
}

/// Generate HTML code for a colorful representation of an [AccessRole] on Passphrase management
/// sites
pub fn format_access_role(role: &AccessRole) -> askama::filters::Safe<String> {
    let (icon, color) = match role {
        AccessRole::User => ("person-fill", "primary"),
        AccessRole::Orga => ("clipboard", "warning"),
        AccessRole::Admin | AccessRole::ServerAdmin => ("gear-fill", "warning"),
        AccessRole::SharableViewLink => ("share", "info"),
    };
    askama::filters::Safe(format!(
        "<span class=\"text-{}\"><i class=\"bi bi-{}\"></i> {}</span>",
        color,
        icon,
        role.name()
    ))
}

pub fn validate_optional_datetime_local_value<T: chrono::TimeZone>(
    value: &mut FormValue<validation::MaybeEmpty<validation::DateTimeLocal>>,
    local_timezone: &T,
) -> Option<Option<chrono::DateTime<chrono::Utc>>> {
    let local_datetime = value.validate()?;
    if let Some(local_datetime) = local_datetime.0 {
        let utc_datetime = local_timezone
            .from_local_datetime(&local_datetime.0)
            .latest()
            .map(|v| v.to_utc());
        if let Some(utc_datetime) = utc_datetime {
            Some(Some(utc_datetime))
        } else {
            value.add_error("This point in time does not exist in the local timezone.".to_owned());
            None
        }
    } else {
        Some(None)
    }
}
