use crate::data_store::EventFilter;
use crate::web::api::APIError;
use crate::web::AppState;
use actix_web::{get, web, Responder};
use serde::{Deserialize, Serialize};

#[get("/events")]
async fn list_events(
    query: web::Query<EventFilterAsQuery>,
    state: web::Data<AppState>,
) -> Result<impl Responder, APIError> {
    let event: Vec<kueaplan_api_types::Event> = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        Ok(store.get_events(query.into_inner().into())?)
    })
    .await??
    .into_iter()
    .map(|e| e.into())
    .collect();
    Ok(web::Json(event))
}

#[derive(Deserialize, Serialize, Default)]
struct EventFilterAsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    after: Option<chrono::NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    before: Option<chrono::NaiveDate>,
}

impl From<EventFilterAsQuery> for EventFilter {
    fn from(value: EventFilterAsQuery) -> Self {
        Self {
            after: value.after,
            before: value.before,
        }
    }
}

#[get("/events/{event_id}")]
async fn get_event_info(
    path: web::Path<i32>,
    state: web::Data<AppState>,
) -> Result<impl Responder, APIError> {
    let event_id = path.into_inner();
    let event: kueaplan_api_types::Event = web::block(move || -> Result<_, APIError> {
        let mut store = state.store.get_facade()?;
        Ok(store.get_event(event_id)?)
    })
    .await??
    .into();
    Ok(web::Json(event))
}
