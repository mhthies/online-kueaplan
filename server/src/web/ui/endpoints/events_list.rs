use crate::data_store::models::Event;
use crate::data_store::EventFilter;
use crate::web::ui::base_template::BaseTemplateContext;
use crate::web::ui::error::AppError;
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use askama::Template;

#[get("/events")]
async fn events_list(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let events = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let filter = EventFilter::builder()
            .after(chrono::Utc::now().date_naive() - chrono::Duration::days(10))
            .before(chrono::Utc::now().date_naive() + chrono::Duration::days(10))
            .build();
        Ok(store.get_events(filter)?)
    })
    .await??;

    let tmpl = EventsListTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Veranstaltungen",
            event: None,
            current_date: None,
            auth_token: None,
            active_main_nav_button: None,
        },
        events: &events,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "events_list.html")]
struct EventsListTemplate<'a> {
    base: BaseTemplateContext<'a>,
    events: &'a Vec<Event>,
}
