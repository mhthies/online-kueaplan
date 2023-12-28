use std::{fmt::Display, vec::Vec};

use actix_web::{
    error::ResponseError,
    get,
    http::{header::ContentType, StatusCode},
    middleware, put, web, App, HttpResponse, HttpServer,
};
use serde_json::json;
use uuid::Uuid;

use kueaplan_backend::database::models::*;
use kueaplan_backend::database::{KueaPlanStore, StoreError};

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
                "status": self.status_code().as_u16(),
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
struct AppState {
    db_pool: kueaplan_backend::database::DbPool,
}

impl AppState {
    fn new() -> Self {
        Self {
            db_pool: kueaplan_backend::database::DbPool::new(),
        }
    }
}

#[get("/event/{event_id}/entries")]
async fn list_entries(
    path: web::Path<i32>,
    state: web::Data<AppState>,
) -> web::Json<Vec<FullEntry>> {
    web::Json(vec![]) // TODO
}

#[get("/event/{event_id}/entries/{entry_id}")]
async fn get_entry(
    path: web::Path<(i32, Uuid)>,
    state: web::Data<AppState>,
) -> Result<web::Json<FullEntry>, APIError> {
    let (_event_id, entry_id) = path.into_inner();
    let entry = web::block(move || state.db_pool.get_store().get_entry(entry_id)).await??;
    Ok(web::Json(entry))
}

#[put("/event/{event_id}/entries/{entry_id}")]
async fn create_or_update_entry(
    path: web::Path<(i32, Uuid)>,
    data: web::Json<FullEntry>,
    state: web::Data<AppState>,
) -> Result<&'static str, APIError> {
    let (_event_id, _entry_id) = path.into_inner(); // TODO check?
    web::block(move || state.db_pool.get_store().create_entry(data.0)).await??;

    Ok("")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = AppState::new();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(middleware::Compress::default())
            .service(list_entries)
            .service(get_entry)
            .service(create_or_update_entry)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
