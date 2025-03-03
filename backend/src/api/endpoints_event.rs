use super::{APIError, AppState, SessionTokenHeader};
use actix_web::{get, web, Responder};

#[get("/events/{event_id}")]
async fn get_event_info(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let event_id = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let event: kueaplan_api_types::Event = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.check_authorization(&session_token, event_id)?;
        Ok(store.get_event(&auth, event_id)?)
    })
    .await??
    .into();
    Ok(web::Json(event))
}
