use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, Event, FullEntry, Room};
use crate::data_store::EntryId;
use crate::web::ui::base_template::BaseTemplateContext;
use crate::web::ui::error::AppError;
use crate::web::ui::flash::{FlashMessage, FlashType, FlashesInterface};
use crate::web::ui::sub_templates::edit_entry_helpers::{
    EditEntryNavbar, EditEntryNavbarActiveLink,
};
use crate::web::ui::sub_templates::main_list_row::{
    styles_for_category, MainListRow, MainListRowTemplate,
};
use crate::web::ui::time_calculation::{get_effective_date, TIME_ZONE};
use crate::web::ui::util;
use crate::web::AppState;
use actix_web::web::{Html, Redirect};
use actix_web::{get, post, web, HttpRequest, Responder};
use askama::Template;
use std::collections::BTreeMap;

#[get("/{event_id}/entry/{entry_id}/previous_dates")]
async fn previous_dates_overview(
    path: web::Path<(i32, EntryId)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, entry_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageEntries, event_id)?;
    let store = state.store.clone();
    let (entry, event, rooms, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageEntries)?;
        Ok((
            store.get_entry(&auth, entry_id)?,
            store.get_event(event_id)?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?, // TODO only get relevant category?
            auth,
        ))
    })
    .await??;

    let tmpl = PreviousDatesOverviewTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Vorherige Termine",
            event: Some(&event),
            current_date: Some(get_effective_date(&entry.entry.begin)),
            auth_token: Some(&auth),
            active_main_nav_button: None,
        },
        event: &event,
        entry: &entry,
        rooms: rooms.iter().map(|r| (r.id, r)).collect(),
        entry_category: categories
            .iter()
            .filter(|c| c.id == entry.entry.category)
            .next()
            .ok_or(AppError::InternalError(format!(
                "Entry's category {} does not exist.",
                entry.entry.category
            )))?,
        timezone: TIME_ZONE,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/entry/{entry_id}/previous_dates/{previous_date_id}/delete")]
async fn delete_previous_date(
    path: web::Path<(i32, EntryId, uuid::Uuid)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, entry_id, previous_date_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageEntries, event_id)?;

    let result = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        // TODO add explicit delete function to the data_store interface and use it here instead of
        //   reading + updating entry
        let mut entry = store.get_entry(&auth, entry_id)?;
        let last_updated = entry.entry.last_updated;
        let previous_date_index = entry
            .previous_dates
            .iter()
            .position(|pd| pd.previous_date.id == previous_date_id)
            .ok_or(AppError::EntityNotFound)?;
        let removed_previous_date = entry.previous_dates.swap_remove(previous_date_index);
        store.create_or_update_entry(&auth, entry.into(), false, Some(last_updated))?;
        Ok(removed_previous_date)
    })
    .await?;

    let notification = match result {
        Ok(_) => FlashMessage {
            flash_type: FlashType::Success,
            message: "Der vorherige Termin wurde gelöscht.".to_string(),
            keep_open: false,
            button: None,
        },
        Err(e) => match e {
            AppError::TransactionConflict => FlashMessage {
                flash_type: FlashType::Error,
                message: "Der vorherige Termin konnte wegen eines parallelen Datenbank-Zugriff nicht gelöscht werden. Bitte erneut versuchen.".to_string(),
                keep_open: true,
                button: None,
            },
            AppError::ConcurrentEditConflict => FlashMessage {
                flash_type: FlashType::Error,
                message: "Der vorherige Termin konnte wegen einer parallelen Änderung nicht gelöscht werden. Bitte erneut versuchen.".to_string(),
                keep_open: true,
                button: None,
            },
            _ => return Err(e.into()),
        },
    };
    req.add_flash_message(notification);

    Ok(Redirect::to(
        req.url_for(
            "previous_dates_overview",
            &[event_id.to_string(), entry_id.to_string()],
        )?
        .to_string(),
    )
    .see_other())
}

#[derive(Template)]
#[template(path = "previous_dates_overview.html")]
struct PreviousDatesOverviewTemplate<'a> {
    base: BaseTemplateContext<'a>,
    event: &'a Event,
    entry: &'a FullEntry,
    rooms: BTreeMap<uuid::Uuid, &'a Room>,
    entry_category: &'a Category,
    timezone: chrono_tz::Tz,
}

impl PreviousDatesOverviewTemplate<'_> {
    fn to_our_timezone(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
        timestamp.with_timezone(&self.timezone).naive_local()
    }
}
