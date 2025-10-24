use crate::data_store::models::ExtendedEvent;
use crate::data_store::EventId;
use crate::web::api::{APIError, SessionTokenHeader};
use crate::web::AppState;
use actix_web::{get, put, web, HttpResponse, Responder};

#[get("/events/{event_id}/extended")]
async fn get_extended_event_info(
    path: web::Path<EventId>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let event_id = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let event: kueaplan_api_types::ExtendedEvent = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok(store.get_extended_event(&auth, event_id)?)
    })
    .await??
    .into();
    Ok(web::Json(event))
}

#[put("/events/{event_id}/extended")]
async fn update_extended_event(
    path: web::Path<EventId>,
    data: web::Json<kueaplan_api_types::ExtendedEvent>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let event_id = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let event = data.into_inner();
    if event_id != event.basic_data.id {
        return Err(APIError::EntityIdMissmatch);
    }
    web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        store.update_event(
            &auth,
            ExtendedEvent::try_from(event).map_err(|e| APIError::InvalidData(e.to_string()))?,
        )?;
        Ok(())
    })
    .await??;

    Ok(HttpResponse::NoContent())
}
