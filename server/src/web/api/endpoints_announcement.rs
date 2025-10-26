use crate::data_store::models::FullNewAnnouncement;
use crate::web::api::{APIError, SessionTokenHeader};
use crate::web::AppState;
use actix_web::{delete, get, patch, put, web, HttpResponse, Responder};
use uuid::Uuid;

#[get("/events/{event_id}/announcements")]
async fn list_announcements(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let event_id = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let announcements: Vec<kueaplan_api_types::Announcement> =
        web::block(move || -> Result<_, APIError> {
            let mut store = state.store.get_facade()?;
            let auth = store.get_auth_token_for_session(&session_token, event_id)?;
            Ok(store.get_announcements(&auth, event_id, None)?)
        })
        .await??
        .into_iter()
        .map(|e| e.into())
        .collect();

    Ok(web::Json(announcements))
}

#[put("/events/{event_id}/announcements/{announcement_id}")]
async fn create_or_update_announcement(
    path: web::Path<(i32, Uuid)>,
    data: web::Json<kueaplan_api_types::Announcement>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let (event_id, announcement_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let announcement = data.into_inner();
    if announcement_id != announcement.id {
        return Err(APIError::EntityIdMissmatch);
    }
    let created = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok(store.create_or_update_announcement(
            &auth,
            FullNewAnnouncement::from_api(announcement, event_id),
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

#[patch("/events/{event_id}/announcements/{announcement_id}")]
async fn change_announcement(
    path: web::Path<(i32, Uuid)>,
    data: web::Json<kueaplan_api_types::AnnouncementPatch>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let (event_id, announcement_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let announcement = data.into_inner();
    web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok(store.patch_announcement(&auth, announcement_id, announcement.into())?)
    })
    .await??;

    Ok(HttpResponse::NoContent())
}

#[delete("/events/{event_id}/announcements/{announcement_id}")]
async fn delete_announcement(
    path: web::Path<(i32, Uuid)>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let (event_id, announcement_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    // TODO allow replacing announcement
    web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        store.delete_announcement(&auth, event_id, announcement_id)?;
        Ok(())
    })
    .await??;

    Ok(HttpResponse::NoContent())
}
