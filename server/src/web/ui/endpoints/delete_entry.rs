use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, Event, FullEntry, Room};
use crate::data_store::EntryId;
use crate::web::time_calculation;
use crate::web::time_calculation::get_effective_date;
use crate::web::ui::base_template::BaseTemplateContext;
use crate::web::ui::error::AppError;
use crate::web::ui::flash::{FlashMessage, FlashType, FlashesInterface};
use crate::web::ui::sub_templates::edit_entry_helpers::{
    EditEntryNavbar, EditEntryNavbarActiveLink,
};
use crate::web::ui::sub_templates::main_list_row::{
    styles_for_category, MainEntryLinkMode, MainListRow, MainListRowTemplate,
};
use crate::web::ui::util;
use crate::web::AppState;
use actix_web::web::{Html, Redirect};
use actix_web::{get, post, web, HttpRequest, Responder};
use askama::Template;
use std::collections::BTreeMap;

#[get("/{event_id}/entry/{entry_id}/delete")]
async fn delete_entry_form(
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

    let tmpl = DeleteEntryTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Eintrag löschen",
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
            .find(|c| c.id == entry.entry.category)
            .ok_or(AppError::InternalError(format!(
                "Entry's category {} does not exist.",
                entry.entry.category
            )))?,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/entry/{entry_id}/delete")]
async fn delete_entry(
    path: web::Path<(i32, EntryId)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, entry_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageEntries, event_id)?;

    let result = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        let entry = store.get_entry(&auth, entry_id)?;
        store.delete_entry(&auth, event_id, entry_id)?;
        Ok(entry.entry.begin)
    })
    .await?;

    match result {
        Ok(entry_begin) => {
            let notification = FlashMessage {
                flash_type: FlashType::Success,
                message: "Der Eintrag wurde gelöscht.".to_string(),
                keep_open: false,
                button: None,
            };
            req.add_flash_message(notification);
            Ok(Redirect::to(util::url_for_main_list(
                &req,
                event_id,
                &time_calculation::get_effective_date(&entry_begin),
            )?)
            .see_other())
        }
        Err(e) => match e {
            AppError::TransactionConflict => {
                let notification = FlashMessage {
                flash_type: FlashType::Error,
                message: "Der Eintrag konnte wegen eines parallelen Datenbank-Zugriff nicht gelöscht werden. Bitte erneut versuchen.".to_string(),
                keep_open: true,
                button: None,
            };
                req.add_flash_message(notification);
                Ok(Redirect::to(
                    req.url_for(
                        "delete_entry_form",
                        &[event_id.to_string(), entry_id.to_string()],
                    )?
                    .to_string(),
                )
                .see_other())
            }
            _ => Err(e),
        },
    }
}

#[post("/{event_id}/entry/{entry_id}/mark_cancelled")]
async fn mark_entry_cancelled(
    path: web::Path<(i32, EntryId)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, entry_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageEntries, event_id)?;

    let result = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        // TODO add explicit function to the data_store interface and use it here instead of
        //   reading + updating entry
        let mut entry = store.get_entry(&auth, entry_id)?;
        let last_updated = entry.entry.last_updated;
        let entry_begin = entry.entry.begin;
        entry.entry.is_cancelled = true;
        store.create_or_update_entry(&auth, entry.into(), false, Some(last_updated))?;
        Ok(entry_begin)
    })
    .await?;

    match result {
        Ok(entry_begin) => {
            let notification = FlashMessage {
                flash_type: FlashType::Success,
                message: "Die Änderung wurde gespeichert.".to_string(),
                keep_open: false,
                button: None,
            };
            req.add_flash_message(notification);

            Ok(Redirect::to(
                util::url_for_entry_details(
                    &req,
                    event_id,
                    &entry_id,
                    &time_calculation::get_effective_date(&entry_begin),
                )?
                .to_string(),
            )
            .see_other())
        }
        Err(e) => {
            let notification = match e {
                AppError::TransactionConflict => FlashMessage {
                    flash_type: FlashType::Error,
                    message: "Der Eintrag konnte wegen eines parallelen Datenbank-Zugriff nicht geändert werden. Bitte erneut versuchen.".to_string(),
                    keep_open: true,
                    button: None,
                },
                AppError::ConcurrentEditConflict => FlashMessage {
                    flash_type: FlashType::Error,
                    message: "Der Eintrag konnte wegen einer parallelen Änderung nicht geändert werden. Bitte erneut versuchen.".to_string(),
                    keep_open: true,
                    button: None,
                },
                _ => return Err(e),
            };

            req.add_flash_message(notification);
            Ok(Redirect::to(
                req.url_for(
                    "delete_entry_form",
                    &[event_id.to_string(), entry_id.to_string()],
                )?
                .to_string(),
            )
            .see_other())
        }
    }
}

#[derive(Template)]
#[template(path = "delete_entry_form.html")]
struct DeleteEntryTemplate<'a> {
    base: BaseTemplateContext<'a>,
    event: &'a Event,
    entry: &'a FullEntry,
    rooms: BTreeMap<uuid::Uuid, &'a Room>,
    entry_category: &'a Category,
}
