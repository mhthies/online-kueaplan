use crate::web::api::{APIError, SessionTokenHeader};
use crate::web::AppState;
use actix_web::{get, web, Responder};

#[get("/events/{event_id}/extended")]
async fn get_extended_event_info(
    path: web::Path<i32>,
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
