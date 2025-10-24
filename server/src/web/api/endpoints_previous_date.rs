use crate::data_store::models::FullPreviousDate;
use crate::web::api::{APIError, SessionTokenHeader};
use crate::web::AppState;
use actix_web::{delete, put, web, HttpResponse, Responder};
use uuid::Uuid;

#[put("/events/{event_id}/entries/{entry_id}/previousDates/{previous_date_id}")]
async fn create_or_update_previous_date(
    path: web::Path<(i32, Uuid, Uuid)>,
    data: web::Json<kueaplan_api_types::PreviousDate>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let (event_id, entry_id, previous_date_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let previous_date = data.into_inner();
    if previous_date_id != previous_date.id {
        return Err(APIError::EntityIdMissmatch);
    }
    let created = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok(store.create_or_update_previous_date(
            &auth,
            FullPreviousDate::from_api(previous_date, entry_id),
        )?)
    })
    .await??;

    if created {
        Ok(HttpResponse::Created())
    } else {
        Ok(HttpResponse::NoContent())
    }
}

#[delete("/events/{event_id}/entries/{entry_id}/previousDates/{previous_date_id}")]
async fn delete_previous_date(
    path: web::Path<(i32, Uuid, Uuid)>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let (event_id, entry_id, previous_date_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        store.delete_previous_date(&auth, entry_id, previous_date_id)?;
        Ok(())
    })
    .await??;

    Ok(HttpResponse::NoContent())
}
