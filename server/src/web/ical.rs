use crate::auth_session::SessionToken;
use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, Event, FullEntry, Room};
use crate::data_store::{CategoryId, EntryFilter, RoomId};
use crate::web::ui::error::AppError;
use crate::web::AppState;
use actix_web::http::header::DispositionParam;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponseBuilder, Responder};
use icalendar::{Component, EventLike};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[allow(clippy::identity_op)] // We want to explicitly state that it's "1" year
pub const SESSION_COOKIE_MAX_AGE: std::time::Duration =
    std::time::Duration::from_secs(1 * 86400 * 365);

#[get("/events/{event_id}/ical")]
async fn ical(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    query: web::Query<ICalQueryParams>,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let query = query.into_inner();
    let session_token =
        SessionToken::from_string(&query.session_token, &state.secret, SESSION_COOKIE_MAX_AGE)
            .map_err(|session_error| AppError::PermissionDenied {
                required_privilege: Privilege::ShowKueaPlan,
                event_id,
                session_error: Some(session_error),
            })?;

    let (event, entries, rooms, categories) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok((
            store.get_event(event_id)?,
            store.get_entries_filtered(&auth, event_id, EntryFilter::default())?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
        ))
    })
    .await??;

    Ok(HttpResponseBuilder::new(StatusCode::OK)
        .content_type("text/calendar; charset=utf-8")
        .append_header(actix_web::http::header::ContentDisposition {
            disposition: actix_web::http::header::DispositionType::Inline,
            parameters: vec![DispositionParam::Filename(String::from("kueaplan.ics"))],
        })
        .body(generate_ical(event, entries, rooms, categories)))
}

#[derive(Deserialize, Serialize)]
pub struct ICalQueryParams {
    #[serde(rename = "token")]
    pub session_token: String,
}

fn generate_ical(
    event: Event,
    entries: Vec<FullEntry>,
    rooms: Vec<Room>,
    categories: Vec<Category>,
) -> String {
    let mut calendar = icalendar::Calendar::new()
        .name(&format!("KüA-Plan {}", event.title))
        .done();
    let rooms_by_id: BTreeMap<RoomId, &Room> = rooms.iter().map(|r| (r.id, r)).collect();
    let categories_by_id: BTreeMap<CategoryId, &Category> =
        categories.iter().map(|c| (c.id, c)).collect();

    for entry in entries {
        if entry.entry.is_cancelled {
            continue;
        }

        let mut event = icalendar::Event::new()
            .uid(&entry.entry.id.to_string())
            .summary(&entry.entry.title)
            .starts(entry.entry.begin)
            .ends(entry.entry.end)
            .description(&generate_ical_description(&entry))
            .location(&generate_ical_location(&entry, &rooms_by_id))
            .done();
        if let Some(category) = categories_by_id.get(&entry.entry.category) {
            event.append_property(icalendar::Property::new("CATEGORIES", &category.title));
        }
        calendar.push(event);
    }

    calendar.to_string()
}

fn generate_ical_description(entry: &FullEntry) -> String {
    let mut description = entry.entry.comment.clone();
    if !entry.entry.responsible_person.is_empty() {
        if !description.is_empty() {
            description.push_str("\n");
        }
        description.push_str("von ");
        description.push_str(&entry.entry.responsible_person);
    }
    if !entry.entry.time_comment.is_empty() {
        if !description.is_empty() {
            description.push_str("\n");
        }
        description.push_str(&entry.entry.time_comment);
    }
    if !entry.entry.description.is_empty() {
        if !description.is_empty() {
            description.push_str("\n\n");
        }
        description.push_str(&entry.entry.description);
    }
    description
}

fn generate_ical_location(entry: &FullEntry, rooms: &BTreeMap<RoomId, &Room>) -> String {
    let room_names: Vec<String> = entry
        .room_ids
        .iter()
        .filter_map(|room_id| rooms.get(room_id))
        .map(|r| r.title.clone())
        .collect();

    let mut location = room_names.join("\n");
    if !entry.entry.room_comment.is_empty() {
        if !location.is_empty() {
            location.push_str("; ");
        }
        location.push_str(&entry.entry.room_comment);
    }

    location
}
