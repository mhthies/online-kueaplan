use crate::data_store::models::{EntryState, FullNewEntry, NewEntry};
use crate::web::api::{APIError, SessionTokenHeader};
use crate::web::util::EntryFilterAsQuery;
use crate::web::AppState;
use actix_web::{delete, get, patch, post, put, web, HttpResponse, Responder};
use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer};
use uuid::Uuid;

#[get("/events/{event_id}/entries")]
async fn list_entries(
    path: web::Path<i32>,
    query: web::Query<EntryFilterAsQuery>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let event_id = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let entries: Vec<kueaplan_api_types::Entry> = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok(store.get_published_entries_filtered(&auth, event_id, query.into_inner().into())?)
    })
    .await??
    .into_iter()
    .map(|e| e.into())
    .collect();

    Ok(web::Json(entries))
}

#[get("/events/{event_id}/allEntries")]
async fn list_all_entries(
    path: web::Path<i32>,
    query: web::Query<AllEntriesQuery>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let event_id = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let query_data = query.into_inner();
    let filter = query_data.generic_filter.into();
    let states_filter = query_data
        .state_filter
        .map(|states| -> Vec<EntryState> { states.into_iter().map(Into::into).collect() })
        .unwrap_or(EntryState::all().copied().collect());
    let entries: Vec<kueaplan_api_types::Entry> = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok(store.get_all_entries_filtered(&auth, event_id, filter, &states_filter)?)
    })
    .await??
    .into_iter()
    .map(|e| e.into())
    .collect();

    Ok(web::Json(entries))
}

#[derive(Deserialize, Default)]
pub struct AllEntriesQuery {
    #[serde(flatten)]
    pub generic_filter: EntryFilterAsQuery,
    #[serde(
        rename = "state",
        deserialize_with = "deserialize_optional_comma_separated_list_of_event_states"
    )]
    #[serde(default)]
    pub state_filter: Option<Vec<kueaplan_api_types::EntryState>>,
}

#[get("/events/{event_id}/entries/{entry_id}")]
async fn get_entry(
    path: web::Path<(i32, Uuid)>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let (event_id, entry_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let entry: kueaplan_api_types::Entry = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok(store.get_entry(&auth, entry_id)?)
    })
    .await??
    .into();
    Ok(web::Json(entry))
}

#[put("/events/{event_id}/entries/{entry_id}")]
async fn create_or_update_entry(
    path: web::Path<(i32, Uuid)>,
    data: web::Json<kueaplan_api_types::Entry>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let (event_id, entry_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let entry = data.into_inner();
    if entry_id != entry.id {
        return Err(APIError::EntityIdMissmatch);
    }
    let created = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok(store.create_or_update_entry(
            &auth,
            FullNewEntry::from_api(entry, event_id),
            false,
            None, // TODO allow using E-Tag for conflict checking
        )?)
    })
    .await??;

    if created {
        Ok(HttpResponse::Created())
    } else {
        Ok(HttpResponse::NoContent())
    }
}

#[patch("/events/{event_id}/entries/{entry_id}")]
async fn change_entry(
    path: web::Path<(i32, Uuid)>,
    data: web::Json<kueaplan_api_types::EntryPatch>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let (event_id, entry_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let entry = data.into_inner();
    web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok(store.patch_entry(&auth, entry_id, entry.into())?)
    })
    .await??;

    Ok(HttpResponse::NoContent())
}

#[post("/events/{event_id}/submitEntry")]
async fn submit_entry(
    path: web::Path<i32>,
    data: web::Json<kueaplan_api_types::EntrySubmission>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let event_id = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let submission = data.into_inner();
    let entry = FullNewEntry {
        entry: NewEntry {
            id: submission.id,
            title: submission.title,
            description: submission.description,
            responsible_person: submission.responsible_person,
            is_room_reservation: submission.is_room_reservation,
            event_id,
            begin: submission.begin,
            end: submission.end,
            category: submission.category,
            comment: submission.comment,
            time_comment: submission.time_comment,
            room_comment: submission.room_comment,
            is_exclusive: false,
            is_cancelled: false,
            state: if submission.publish_without_review {
                EntryState::PreliminaryPublished
            } else {
                EntryState::SubmittedForReview
            },
            orga_comment: if submission.submitter_comment.is_empty() {
                "".to_string()
            } else {
                format!(
                    "Kommentar der einreichenden Person:\n> {}",
                    submission.submitter_comment.replace("\n", "\n> ")
                )
            },
        },
        room_ids: submission.room,
        previous_dates: vec![],
    };
    web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        store.submit_entry_by_participant(&auth, entry)?;
        Ok(())
    })
    .await??;
    Ok(HttpResponse::Ok())
}

#[delete("/events/{event_id}/entries/{entry_id}")]
async fn delete_entry(
    path: web::Path<(i32, Uuid)>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let (event_id, entry_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        store.delete_entry(&auth, event_id, entry_id)?;
        Ok(())
    })
    .await?
    .map_err(APIError::for_delete_endpoint)?;

    Ok(HttpResponse::NoContent())
}

fn deserialize_optional_comma_separated_list_of_event_states<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<kueaplan_api_types::EntryState>>, D::Error>
where
    D: Deserializer<'de>,
{
    let str_sequence = String::deserialize(deserializer)?;
    let result = str_sequence
        .split(',')
        .filter(|s| !s.is_empty())
        .map(serde_urlencoded::from_str)
        .collect::<Result<Vec<kueaplan_api_types::EntryState>, _>>()
        .map_err(|_| {
            D::Error::invalid_value(
                Unexpected::Str(&str_sequence),
                &"A comma-separated list of entry state names",
            )
        })?;
    Ok(Some(result))
}
