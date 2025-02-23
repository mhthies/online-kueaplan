use std::sync::Arc;
use std::{env, fmt::Display, vec::Vec};

#[cfg(test)]
mod tests;

use crate::auth_session::SessionToken;
use crate::data_store::models::*;
use crate::data_store::{get_store_from_env, StoreError};
use actix_web::{
    error::ResponseError,
    get,
    http::{header::ContentType, StatusCode},
    put, web, HttpResponse,
};
use serde_json::json;
use uuid::Uuid;

pub fn configure_app(cfg: &mut web::ServiceConfig) {
    cfg.service(get_api_service());
}

fn get_api_service() -> actix_web::Scope {
    web::scope("/api/v1")
        .service(list_entries)
        .service(get_entry)
        .service(create_or_update_entry)
}

#[derive(Debug)]
enum APIError {
    NotExisting,
    PermissionDenied,
    NoSessionToken,
    InvalidSessionToken,
    BackendError(String),
    InternalError(String),
}

impl Display for APIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotExisting => f.write_str("Element does not exist")?,
            Self::PermissionDenied => {
                f.write_str("Client is not authorized to perform this action")?
            },
            Self::NoSessionToken => {
                f.write_str("This action requires authentication, but client did not send authentication session token.")?
            },
            Self::InvalidSessionToken => {
                f.write_str("This action requires authentication, but client authentication session given by the client is not valid.")?
            },
            Self::BackendError(s) => {
                f.write_str("Database error: ")?;
                f.write_str(s)?;
            },
            Self::InternalError(s) => {
                f.write_str("Internal error: ")?;
                f.write_str(s)?;
            },
        };
        Ok(())
    }
}

impl ResponseError for APIError {
    fn error_response(&self) -> HttpResponse {
        let message = format!("{}", self);

        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(json!({
                "httpCode": self.status_code().as_u16(),
                "message": message
            }))
    }
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotExisting => StatusCode::NOT_FOUND,
            Self::PermissionDenied => StatusCode::FORBIDDEN,
            Self::NoSessionToken => StatusCode::FORBIDDEN,
            Self::InvalidSessionToken => StatusCode::FORBIDDEN,
            Self::BackendError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<StoreError> for APIError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::ConnectionPoolError(r2d2_error) => Self::BackendError(format!(
                "Could not get database connection from pool: {}",
                r2d2_error
            )),
            StoreError::ConnectionError(diesel_error) => {
                Self::BackendError(diesel_error.to_string())
            }
            StoreError::QueryError(diesel_error) => Self::BackendError(diesel_error.to_string()),
            StoreError::NotExisting => Self::NotExisting,
            StoreError::PermissionDenied => Self::PermissionDenied,
            StoreError::InvalidSession => Self::InvalidSessionToken,
            StoreError::InvalidData => Self::InternalError("Invalid data".to_owned()),
        }
    }
}

impl From<actix_web::error::BlockingError> for APIError {
    fn from(_e: actix_web::error::BlockingError) -> Self {
        APIError::InternalError(
            "Could not get thread from thread pool for synchronous database operation.".to_owned(),
        )
    }
}

impl From<crate::auth_session::SessionError> for APIError {
    fn from(_e: crate::auth_session::SessionError) -> Self {
        APIError::InvalidSessionToken
    }
}

#[derive(Clone)]
pub struct AppState {
    store: Arc<dyn crate::data_store::KuaPlanStore>,
    secret: String,
}

impl AppState {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            store: Arc::new(get_store_from_env()?),
            secret: env::var("SECRET").map_err(|_| "SECRET must be set")?.into(),
        })
    }
}

struct SessionTokenHeader(String);

const SESSION_TOKEN_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(1 * 86400 * 365);

impl SessionTokenHeader {
    fn session_token(
        &self,
        secret: &str,
    ) -> Result<crate::auth_session::SessionToken, crate::auth_session::SessionError> {
        SessionToken::from_string(&self.0, secret, SESSION_TOKEN_MAX_AGE)
    }
}

impl actix_web::http::header::TryIntoHeaderValue for SessionTokenHeader {
    type Error = actix_web::http::header::InvalidHeaderValue;

    fn try_into_value(self) -> Result<actix_web::http::header::HeaderValue, Self::Error> {
        Ok(self.0.parse()?)
    }
}

impl actix_web::http::header::Header for SessionTokenHeader {
    fn name() -> actix_web::http::header::HeaderName {
        "X-SESSION-TOKEN"
            .try_into()
            .expect("Session Token Header name should be a valid header name")
    }

    fn parse<M: actix_web::HttpMessage>(msg: &M) -> Result<Self, actix_web::error::ParseError> {
        Ok(Self(
            msg.headers()
                .get(Self::name())
                .ok_or(actix_web::error::ParseError::Header)?
                .to_str()
                .unwrap_or("")
                .to_owned(),
        ))
    }
}

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
        Ok(store.create_entry(&auth, FullNewEntry::from_api(data.into_inner(), event_id))?)
    })
    .await??;

    Ok("")
}
