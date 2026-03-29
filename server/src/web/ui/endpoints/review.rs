use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, EntryState, ExtendedEvent, FullEntry};
use crate::data_store::EntryFilter;
use crate::web::ui::base_template::{AnyEventData, BaseTemplateContext, MainNavButton};
use crate::web::ui::error::AppError;
use crate::web::ui::sub_templates::main_list_row::{
    MainListRow, MainListRowTemplate, RoomByIdWithOrder,
};
use crate::web::ui::util;
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use askama::Template;
use std::collections::BTreeMap;

#[get("/{event_id}/review/to_review")]
async fn list_to_review(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    generic_review_list(
        path,
        state,
        req,
        &[
            EntryState::PreliminaryPublished,
            EntryState::SubmittedForReview,
        ],
        "Zu prüfende Einträge",
        "Aktuell stehen keine KüA-Einreichungen zur Prüfung aus.",
        ReviewNavButton::ToReview,
    )
    .await
}

#[get("/{event_id}/review/drafts")]
async fn list_drafts(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    generic_review_list(
        path,
        state,
        req,
        &[EntryState::Draft],
        "Entwürfe",
        "Zur Zeit gibt es keine unveröffentlichten Entwürfe.",
        ReviewNavButton::Drafts,
    )
    .await
}

#[get("/{event_id}/review/rejected")]
async fn list_rejected_entries(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    generic_review_list(
        path,
        state,
        req,
        &[EntryState::Rejected],
        "Abgelehnte Einreichungen",
        "Bislang wurden keine Einreichungen abgelehnt (ohne sie zu löschen).",
        ReviewNavButton::Rejected,
    )
    .await
}

#[get("/{event_id}/review/retracted")]
async fn list_retracted_entries(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    generic_review_list(
        path,
        state,
        req,
        &[EntryState::Retracted],
        "Versteckte Einträge",
        "Bislang wurden keine Einträge nach der Veröffentlichung zurückgezogen (ohne sie zu löschen).",
        ReviewNavButton::Retracted,
    )
    .await
}

async fn generic_review_list(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    req: HttpRequest,
    entry_states: &'static [EntryState],
    title: &str,
    empty_message: &str,
    active_nav_button: ReviewNavButton,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageEntries, event_id)?;
    let (entries, entry_count_by_state, rooms, categories, event, auth) =
        web::block(move || -> Result<_, AppError> {
            let mut store = state.store.get_facade()?;
            let auth = store.get_auth_token_for_session(&session_token, event_id)?;
            auth.check_privilege(event_id, Privilege::ManageEntries)?;
            let event = store.get_extended_event(&auth, event_id)?;
            Ok((
                store.get_all_entries_filtered(
                    &auth,
                    event_id,
                    EntryFilter::default(),
                    entry_states,
                )?,
                store.get_entry_count_by_state(&auth, event_id)?,
                store.get_rooms(&auth, event_id)?,
                store.get_categories(&auth, event_id)?,
                event,
                auth,
            ))
        })
        .await??;

    let tmpl = ReviewListTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: title,
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Review),
        },
        entries: &entries,
        rooms: rooms.iter().collect(),
        categories: categories.iter().map(|r| (r.id, r)).collect(),
        active_nav_button,
        event: &event,
        entry_count_by_state: entry_count_by_state.iter().copied().collect(),
        empty_message,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "review.html")]
struct ReviewListTemplate<'a> {
    base: BaseTemplateContext<'a>,
    entries: &'a Vec<FullEntry>,
    rooms: RoomByIdWithOrder<'a>,
    categories: BTreeMap<uuid::Uuid, &'a Category>,
    active_nav_button: ReviewNavButton,
    event: &'a ExtendedEvent,
    entry_count_by_state: BTreeMap<EntryState, i64>,
    empty_message: &'a str,
}

#[derive(Debug, PartialEq)]
pub enum ReviewNavButton {
    ToReview,
    Drafts,
    Rejected,
    Retracted,
}
