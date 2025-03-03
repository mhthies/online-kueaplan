use super::{APIError, AppState, SessionTokenHeader};
use crate::data_store::models::NewRoom;
use actix_web::{delete, get, put, web, HttpResponse, Responder};
use uuid::Uuid;

#[get("/events/{event_id}/rooms")]
async fn list_rooms(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let event_id = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let rooms: Vec<kueaplan_api_types::Room> = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.check_authorization(&session_token, event_id)?;
        Ok(store.get_rooms(&auth, event_id)?)
    })
    .await??
    .into_iter()
    .map(|e| e.into())
    .collect();

    Ok(web::Json(rooms))
}

#[put("/events/{event_id}/rooms/{room_id}")]
async fn create_or_update_room(
    path: web::Path<(i32, Uuid)>,
    data: web::Json<kueaplan_api_types::Room>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let (event_id, room_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let room = data.into_inner();
    if room_id != room.id {
        return Err(APIError::EntityIdMissmatch);
    }
    let created = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.check_authorization(&session_token, event_id)?;
        Ok(store.create_or_update_room(&auth, NewRoom::from_api(room, event_id))?)
    })
    .await??;

    if created {
        Ok(HttpResponse::Created())
    } else {
        Ok(HttpResponse::NoContent())
    }
}

#[delete("/events/{event_id}/rooms/{room_id}")]
async fn delete_room(
    path: web::Path<(i32, Uuid)>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let (event_id, room_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.check_authorization(&session_token, event_id)?;
        store.delete_room(&auth, event_id, room_id)?;
        Ok(())
    })
    .await??;

    Ok(HttpResponse::NoContent())
}
