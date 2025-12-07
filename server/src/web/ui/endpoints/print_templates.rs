use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Event, ExtendedEvent, Passphrase};
use crate::data_store::EventId;
use crate::web::ui::base_template::{
    AnyEventData, BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::util;
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, mime, web, HttpRequest, HttpResponse, Responder};
use askama::Template;
use qrcode::render::svg;
use qrcode::QrCode;

#[get("/{event_id}/config/print_template_link")]
pub async fn print_link_and_passphrase(
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

    let tmpl = PrintLinkAndPassphraseTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Link-Druckvorlage",
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::PrintTemplates,
        },
        event: &event,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "print_link_and_passphrase.html")]
struct PrintLinkAndPassphraseTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    event: &'a ExtendedEvent,
}

impl PrintLinkAndPassphraseTemplate<'_> {
    fn get_ui_link(&self) -> Result<String, AppError> {
        let url = if let Some(slug) = self.event.basic_data.slug.as_ref() {
            self.base
                .request
                .url_for("event_redirect_by_slug", [slug])?
        } else {
            self.base
                .request
                .url_for("event_index", [&self.event.basic_data.id.to_string()])?
        };
        Ok(url.to_string())
    }
}

#[get("/{event_id}/link_qr.svg")]
pub async fn event_ui_link_qr_code(
    path: web::Path<EventId>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();

    let event = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let event = store.get_event(event_id)?;
        Ok(event)
    })
    .await??;
    let url = if let Some(slug) = event.slug {
        req.url_for("event_redirect_by_slug", [slug])?
    } else {
        req.url_for("event_index", [&event_id.to_string()])?
    };

    let code = QrCode::new(url.to_string().as_bytes()).unwrap();
    let image = code
        .render()
        .min_dimensions(200, 200)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build();

    Ok(HttpResponse::Ok()
        .content_type(mime::IMAGE_SVG)
        .message_body(image))
}
