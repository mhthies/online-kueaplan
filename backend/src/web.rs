use std::vec::Vec;

#[macro_use]
extern crate rocket;
use diesel::PgConnection;
use rocket::response::Responder;
use serde_json::json;
use rocket::serde::json::Json;
use uuid::Uuid;
use rocket_sync_db_pools::{database, diesel};

use kueaplan_backend::models::*;
use kueaplan_backend::store::{DataStore, StoreError};

#[database("database")]
struct Database(PgConnection);

#[derive(Debug)]
enum APIError {
    NotExisting,
    BackendError(String)
}

impl<'r> Responder<'r, 'r> for APIError {
    fn respond_to(self, request: &rocket::Request) -> rocket::response::Result<'r> {
        let (status, message) = match self {
            Self::NotExisting => (rocket::http::Status::NotFound, "Element does not exist".to_owned()),
            Self::BackendError(s) => (rocket::http::Status::InternalServerError, s),
        };
        Json(json!({
            "status": status.code,
            "message": message
        }))
        .respond_to(request)
        .map(|mut r| { r.set_status(status); r })
    }
}

impl From<StoreError> for APIError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::ConnectionError(diesel_error) => Self::BackendError(diesel_error.to_string()),
            StoreError::QueryError(diesel_error) => Self::BackendError(diesel_error.to_string()),
            StoreError::NotExisting => Self::NotExisting,
        }
    }
}

#[get("/event/<event_id>/entries")]
fn list_entries(event_id: i32) -> Json<Vec<FullEntry>> {
    Json(vec![]) // TODO
}

#[get("/event/<event_id>/entries/<entry_id>")]
async fn get_entry(db: Database, event_id: i32, entry_id: Uuid) -> Result<Json<FullEntry>, APIError> {
    let entry = db.run(move |connection| DataStore::with_connection(connection).get_entry(entry_id)).await?;
    Ok(Json(entry))
}

#[put("/event/<event_id>/entries/<entry_id>", data = "<data>")]
async fn create_or_update_entry(db: Database, event_id: i32, entry_id: Uuid, data: Json<FullEntry>) -> Result<(), APIError>{
    db.run(move |connection| DataStore::with_connection(connection).create_entry(data.0)).await?;
    Ok(())
}

#[launch]
fn rocket() -> _ {
    rocket::build().attach(Database::fairing()).mount("/", routes![list_entries])
}
