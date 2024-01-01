use std::{fmt::Display, vec::Vec};

use actix_web::{
    error::ResponseError,
    get,
    http::{header::ContentType, StatusCode},
    put, web, HttpResponse,
};
use serde_json::json;
use uuid::Uuid;

use crate::database::models::*;
use crate::database::{KueaPlanStore, StoreError};

pub fn configure_app(cfg: &mut web::ServiceConfig) {
    cfg.service(list_entries)
        .service(get_entry)
        .service(create_or_update_entry);
}

#[derive(Debug)]
enum APIError {
    NotExisting,
    BackendError(String),
    InternalError(String),
}

impl Display for APIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotExisting => f.write_str("Element does not exist")?,
            Self::BackendError(s) => {
                f.write_str("Database error: ")?;
                f.write_str(s)?;
            }
            Self::InternalError(s) => {
                f.write_str("Internal error: ")?;
                f.write_str(s)?;
            }
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

#[derive(Clone)]
pub struct AppState {
    db_pool: crate::database::DbPool,
}

impl AppState {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            db_pool: crate::database::DbPool::new()?,
        })
    }
}

#[get("/event/{event_id}/entries")]
async fn list_entries(
    path: web::Path<i32>,
    state: web::Data<AppState>,
) -> Result<web::Json<Vec<kueaplan_api_types::Entry>>, APIError> {
    let event_id = path.into_inner();
    let entries = web::block(move || state.db_pool.get_store()?.get_entries(event_id))
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
) -> Result<web::Json<kueaplan_api_types::Entry>, APIError> {
    let (_event_id, entry_id) = path.into_inner();
    let entry = web::block(move || state.db_pool.get_store()?.get_entry(entry_id))
        .await??
        .into();
    Ok(web::Json(entry))
}

#[put("/event/{event_id}/entries/{entry_id}")]
async fn create_or_update_entry(
    path: web::Path<(i32, Uuid)>,
    data: web::Json<kueaplan_api_types::Entry>,
    state: web::Data<AppState>,
) -> Result<&'static str, APIError> {
    let (event_id, _entry_id) = path.into_inner(); // TODO check?
    web::block(move || {
        state
            .db_pool
            .get_store()?
            .create_entry(FullNewEntry::from_api(data.into_inner(), event_id))
    })
    .await??;

    Ok("")
}
