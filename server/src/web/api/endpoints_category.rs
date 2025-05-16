use crate::data_store::models::NewCategory;
use crate::web::api::{APIError, SessionTokenHeader};
use crate::web::AppState;
use actix_web::{delete, get, put, web, HttpResponse, Responder};
use uuid::Uuid;

#[get("/events/{event_id}/categories")]
async fn list_categories(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let event_id = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let categories: Vec<kueaplan_api_types::Category> =
        web::block(move || -> Result<_, APIError> {
            let mut store = state.store.get_facade()?;
            let auth = store.get_auth_token_for_session(&session_token, event_id)?;
            Ok(store.get_categories(&auth, event_id)?)
        })
        .await??
        .into_iter()
        .map(|e| e.into())
        .collect();

    Ok(web::Json(categories))
}

#[put("/events/{event_id}/categories/{category_id}")]
async fn create_or_update_category(
    path: web::Path<(i32, Uuid)>,
    data: web::Json<kueaplan_api_types::Category>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let (event_id, category_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let category = data.into_inner();
    if category_id != category.id {
        return Err(APIError::EntityIdMissmatch);
    }
    let created = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok(store.create_or_update_category(&auth, NewCategory::from_api(category, event_id))?)
    })
    .await??;

    if created {
        Ok(HttpResponse::Created())
    } else {
        Ok(HttpResponse::NoContent())
    }
}

#[delete("/events/{event_id}/categories/{category_id}")]
async fn delete_category(
    path: web::Path<(i32, Uuid)>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let (event_id, category_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        store.delete_category(&auth, event_id, category_id, None)?;
        Ok(())
    })
    .await??;

    Ok(HttpResponse::NoContent())
}
