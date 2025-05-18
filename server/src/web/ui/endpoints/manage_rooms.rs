use crate::data_store::auth_token::Privilege;
use crate::data_store::models::Room;
use crate::data_store::EventId;
use crate::web::ui::base_template::{
    BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::util;
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use askama::Template;

#[get("/{event_id}/config/rooms")]
async fn manage_rooms(
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
        auth.check_privilege(event_id, Privilege::ManageRooms)?;
        Ok((
            store.get_event(&auth, event_id)?,
            store.get_rooms(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let tmpl = ManageRoomsTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Orte",
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Rooms,
        },
        event_id,
        rooms: &rooms,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "manage_rooms.html")]
struct ManageRoomsTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    event_id: EventId,
    rooms: &'a Vec<Room>,
}
