use super::{APIError, AppState, SessionTokenHeader};
use crate::data_store::models::FullNewEntry;
use actix_web::{get, put, web};
use uuid::Uuid;

#[get("/event/{event_id}/entries")]
async fn list_entries(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<web::Json<Vec<kueaplan_api_types::Entry>>, APIError> {
    let event_id = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let entries = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.check_authorization(&session_token, event_id)?;
        Ok(store.get_entries(&auth, event_id)?)
    })
    .await??
    .into_iter()
    .map(|e| e.into())
    .collect();

    Ok(web::Json(entries))
}

#[get("/event/{event_id}/entries/{entry_id}")]
async fn get_entry(
    path: web::Path<(i32, Uuid)>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<web::Json<kueaplan_api_types::Entry>, APIError> {
    let (event_id, entry_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let entry = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.check_authorization(&session_token, event_id)?;
        Ok(store.get_entry(&auth, entry_id)?)
    })
    .await??
    .into();
    Ok(web::Json(entry))
}

#[put("/event/{event_id}/entries/{entry_id}")]
async fn create_or_update_entry(
    path: web::Path<(i32, Uuid)>,
    data: web::Json<kueaplan_api_types::Entry>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<&'static str, APIError> {
    let (event_id, _entry_id) = path.into_inner(); // TODO check?
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.check_authorization(&session_token, event_id)?;
        store.create_or_update_entry(&auth, FullNewEntry::from_api(data.into_inner(), event_id))?;
        Ok(())
    })
    .await??;

    Ok("") // TODO return HTTP 201 or 204 respectively
}
