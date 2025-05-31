use crate::data_store::auth_token::Privilege;
use crate::data_store::EntryFilter;
use crate::web::ui::error::AppError;
use crate::web::ui::form_values::ValidateFromFormInput;
use crate::web::ui::time_calculation::{timestamp_from_effective_date_and_time, TIME_ZONE};
use crate::web::ui::{util, validation};
use crate::web::AppState;
use actix_web::{get, web, HttpRequest, Responder};
use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer};
use serde_json::json;

#[get("/{event_id}/concurrent-entries")]
async fn concurrent_entries(
    path: web::Path<i32>,
    query: web::Query<ConcurrentEntriesQuery>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ShowKueaPlan, event_id)?;
    let query = query.into_inner();

    let begin = timestamp_from_effective_date_and_time(query.effective_day, query.begin_time);
    let end = begin + query.duration;
    let filter = EntryFilter::builder().after(begin).before(end).build();

    let entries = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok(store.get_entries_filtered(&auth, event_id, filter)?)
    })
    .await??;

    let mut entries_and_room_conflict_flag: Vec<_> = entries
        .into_iter()
        .map(|e| {
            let has_room_conflict = e.room_ids.iter().any(|r| query.rooms.contains(r));
            (e, has_room_conflict)
        })
        .collect();

    entries_and_room_conflict_flag
        .sort_by_key(|(e, room_conflict)| (!e.entry.is_exclusive, !room_conflict));

    let result: Vec<_> = entries_and_room_conflict_flag
        .into_iter()
        .filter(|(e, _)| !e.entry.is_cancelled)
        .filter(|(e, _)| Some(e.entry.id) != query.current_entry_id)
        .map(|(e, has_room_conflict)| {
            let begin = e.entry.begin.with_timezone(&TIME_ZONE).naive_local();
            let end = e.entry.end.with_timezone(&TIME_ZONE).naive_local();
            let show_begin_date = begin.date() < query.effective_day;
            let show_end_date =
                query.duration > chrono::Duration::hours(12) && end.date() != begin.date();
            json!({
                "title": e.entry.title,
                "begin": if show_begin_date {begin.format("%d.%m. %H:%M").to_string()} else {begin.format("%H:%M").to_string()},
                "end": if show_end_date {end.format("%d.%m. %H:%M").to_string()} else {end.format("%H:%M").to_string()},
                "rooms": e.room_ids,
                "has_room_conflict": has_room_conflict,
                "is_room_reservation": e.entry.is_room_reservation,
                "is_exclusive": e.entry.is_exclusive,
            })
        })
        .collect();

    Ok(web::Json(result))
}

#[derive(Deserialize)]
struct ConcurrentEntriesQuery {
    effective_day: chrono::NaiveDate,
    begin_time: chrono::NaiveTime,
    #[serde(deserialize_with = "deserialize_nice_duration_hours")]
    duration: chrono::Duration,
    #[serde(deserialize_with = "deserialize_comma_separated_list_of_uuids")]
    rooms: Vec<uuid::Uuid>,
    current_entry_id: Option<uuid::Uuid>,
}

fn deserialize_comma_separated_list_of_uuids<'de, D>(
    deserializer: D,
) -> Result<Vec<uuid::Uuid>, D::Error>
where
    D: Deserializer<'de>,
{
    let str_sequence = String::deserialize(deserializer)?;
    str_sequence
        .split(',')
        .filter(|s| !s.is_empty())
        .map(uuid::Uuid::parse_str)
        .collect::<Result<Vec<uuid::Uuid>, uuid::Error>>()
        .map_err(|_| {
            D::Error::invalid_value(
                Unexpected::Str(&str_sequence),
                &"A comma-separated list of uuids",
            )
        })
}

fn deserialize_nice_duration_hours<'de, D>(deserializer: D) -> Result<chrono::Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let duration_string = String::deserialize(deserializer)?;
    let value = validation::NiceDurationHours::from_form_value(&duration_string).map_err(|_| {
        D::Error::invalid_value(
            Unexpected::Str(&duration_string),
            &"A valid 'nice duration'",
        )
    })?;
    Ok(value.into_inner())
}
