use crate::data_store::models::FullNewEntry;
use crate::web::api::{APIError, SessionTokenHeader};
use crate::web::util::EntryFilterAsQuery;
use crate::web::AppState;
use actix_web::{delete, get, put, web, HttpResponse, Responder};
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
        Ok(store.get_entries_filtered(&auth, event_id, query.into_inner().into())?)
    })
    .await??
    .into_iter()
    .map(|e| e.into())
    .collect();

    Ok(web::Json(entries))
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
    .await??;

    Ok(HttpResponse::NoContent())
}
