use crate::data_store::models::NewRoom;
use crate::web::api::{APIError, SessionTokenHeader};
use crate::web::AppState;
use actix_web::{delete, get, put, web, HttpResponse, Responder};
use serde::Deserialize;
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
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
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
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
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
    data: Option<web::Json<DeleteRoomBody>>,
) -> Result<impl Responder, APIError> {
    let (event_id, room_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let data = data.map(web::Json::<_>::into_inner);
    web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        store.delete_room(
            &auth,
            event_id,
            room_id,
            data.as_ref()
                .map(|data| data.replace_rooms.as_slice())
                .unwrap_or(&[]),
            data.as_ref()
                .map(|data| data.add_room_comment.as_str())
                .unwrap_or(""),
        )?;
        Ok(())
    })
    .await?
    .map_err(APIError::for_delete_endpoint)?;

    Ok(HttpResponse::NoContent())
}

#[derive(Deserialize)]
struct DeleteRoomBody {
    #[serde(default, rename = "replaceRooms")]
    replace_rooms: Vec<Uuid>,
    #[serde(default, rename = "addRoomComment")]
    add_room_comment: String,
}
