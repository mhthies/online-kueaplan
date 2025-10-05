use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, FullAnnouncement, Room};
use crate::data_store::EventId;
use crate::web::ui::base_template::{
    AnyEventData, BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::util;
use crate::web::ui::util::{
    announcement_type_color, announcement_type_icon, announcement_type_name,
};
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use askama::Template;
use std::collections::BTreeMap;

#[get("/{event_id}/config/announcements")]
async fn manage_announcements(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ShowKueaPlan, event_id)?;
    let (event, announcements, rooms, categories, auth) =
        web::block(move || -> Result<_, AppError> {
            let mut store = state.store.get_facade()?;
            let auth = store.get_auth_token_for_session(&session_token, event_id)?;
            auth.check_privilege(event_id, Privilege::ManageAnnouncements)?;
            Ok((
                store.get_extended_event(&auth, event_id)?,
                store.get_announcements(&auth, event_id, None)?,
                store.get_rooms(&auth, event_id)?,
                store.get_categories(&auth, event_id)?,
                auth,
            ))
        })
        .await??;

    let tmpl = ManageAnnouncementsTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Bekanntmachungen",
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Announcements,
        },
        event_id,
        announcements: &announcements,
        rooms: rooms.iter().map(|r| (r.id, r)).collect(),
        categories: categories.iter().map(|r| (r.id, r)).collect(),
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "manage_announcements.html")]
struct ManageAnnouncementsTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    event_id: EventId,
    announcements: &'a Vec<FullAnnouncement>,
    rooms: BTreeMap<uuid::Uuid, &'a Room>,
    categories: BTreeMap<uuid::Uuid, &'a Category>,
}

/// Filters for the askama template
mod filters {
    pub use crate::web::ui::askama_filters::markdown;
}
