use crate::web::api::APIError;
use crate::web::AppState;
use actix_web::{get, web, Responder};

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
