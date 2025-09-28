use crate::data_store::models::NewPassphrase;
use crate::web::api::{APIError, SessionTokenHeader};
use crate::web::AppState;
use actix_web::{delete, get, post, web, HttpResponse, Responder};

#[get("/events/{event_id}/passphrases")]
async fn list_passphrases(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let event_id = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let passphrases: Vec<kueaplan_api_types::Passphrase> =
        web::block(move || -> Result<_, APIError> {
            let mut store = state.store.get_facade()?;
            let auth = store.get_auth_token_for_session(&session_token, event_id)?;
            Ok(store.get_passphrases(&auth, event_id)?)
        })
        .await??
        .into_iter()
        .map(|e| e.into())
        .collect();

    Ok(web::Json(passphrases))
}

#[post("/events/{event_id}/passphrases")]
async fn create_passphrase(
    path: web::Path<i32>,
    data: web::Json<kueaplan_api_types::Passphrase>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let event_id = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    let passphrase = data.into_inner();
    if passphrase.id.is_some() {
        return Err(APIError::InvalidData(
            "New passphrase must not have a id field".into(),
        ));
    }
    let passphrase_cloned = passphrase.clone();
    let passphrase_id = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok(store.create_passphrase(&auth, NewPassphrase::from_api(passphrase_cloned, event_id))?)
    })
    .await??;

    let passphrase = kueaplan_api_types::Passphrase {
        id: Some(passphrase_id),
        ..passphrase
    };
    Ok(web::Json(passphrase))
}

#[delete("/events/{event_id}/passphrases/{passphrase_id}")]
async fn delete_passphrase(
    path: web::Path<(i32, i32)>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let (event_id, passphrase_id) = path.into_inner();
    let session_token = session_token_header
        .ok_or(APIError::NoSessionToken)?
        .into_inner()
        .session_token(&state.secret)?;
    web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        store.delete_passphrase(&auth, event_id, passphrase_id)?;
        Ok(())
    })
    .await??;

    Ok(HttpResponse::NoContent())
}
