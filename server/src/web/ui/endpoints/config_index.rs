use crate::data_store::auth_token::Privilege;
use crate::data_store::models::ExtendedEvent;
use crate::web::ui::base_template::{
    AnyEventData, BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::util;
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use askama::Template;

#[get("/{event_id}/config")]
async fn config_index(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ShowConfigArea, event_id)?;
    let (event, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok((store.get_extended_event(&auth, event_id)?, auth))
    })
    .await??;
    auth.check_privilege(event_id, Privilege::ShowConfigArea)?;

    let tmpl = ConfigIndexTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Konfiguration",
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Overview,
        },
        event: &event,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "config_index.html")]
struct ConfigIndexTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    event: &'a ExtendedEvent,
}

impl ConfigIndexTemplate<'_> {
    fn get_shortlink(&self) -> Option<String> {
        self.event.basic_data.slug.as_ref().and_then(|slug| {
            Some(
                self.base
                    .request
                    .url_for("event_redirect_by_slug", [slug])
                    .ok()?
                    .to_string(),
            )
        })
    }
}
