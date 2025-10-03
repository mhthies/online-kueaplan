use crate::data_store::auth_token::Privilege;
use crate::data_store::models::Room;
use crate::data_store::EventId;
use crate::web::ui::base_template::{AnyEventData, BaseTemplateContext, MainNavButton};
use crate::web::ui::error::AppError;
use crate::web::ui::util;
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use askama::Template;

#[get("/{event_id}/rooms")]
async fn rooms_list(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ShowKueaPlan, event_id)?;
    let (event, rooms, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ShowKueaPlan)?;
        Ok((
            store.get_extended_event(&auth, event_id)?,
            store.get_rooms(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let tmpl = RoomsListTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Orte",
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::ByRoom),
        },
        event_id,
        rooms: &rooms,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "rooms_list.html")]
struct RoomsListTemplate<'a> {
    base: BaseTemplateContext<'a>,
    event_id: EventId,
    rooms: &'a Vec<Room>,
}
