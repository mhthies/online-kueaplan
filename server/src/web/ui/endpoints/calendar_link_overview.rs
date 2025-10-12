use crate::data_store::auth_token::Privilege;
use crate::data_store::models::Event;
use crate::data_store::{EventId, StoreError};
use crate::web::ui::base_template::{AnyEventData, BaseTemplateContext};
use crate::web::ui::error::AppError;
use crate::web::ui::util;
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use askama::Template;

#[get("/{event_id}/calendar_links")]
pub async fn calendar_link_overview(
    path: web::Path<EventId>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ShowKueaPlan, event_id)?;
    let store = state.store.clone();
    let (event, shareable_session_token_result, auth) =
        web::block(move || -> Result<_, AppError> {
            let mut store = store.get_facade()?;
            let auth = store.get_auth_token_for_session(&session_token, event_id)?;
            auth.check_privilege(event_id, Privilege::ShowKueaPlan)?;
            Ok((
                store.get_extended_event(&auth, event_id)?,
                store.create_reduced_session_token(
                    &session_token,
                    event_id,
                    Privilege::ShowKueaPlanViaLink,
                ),
                auth,
            ))
        })
        .await??;

    let shareable_session_token = match shareable_session_token_result {
        Ok(token) => Some(token),
        Err(StoreError::NotExisting) => None,
        Err(e) => return Err(e.into()),
    };

    let tmpl = CalendarLinkOverviewTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Links für Kalender-Apps",
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: None,
        },
        shareable_session_token: shareable_session_token.map(|t| t.as_string(&state.secret)),
        event: &event.basic_data,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "calendar_link_overview.html")]
struct CalendarLinkOverviewTemplate<'a> {
    base: BaseTemplateContext<'a>,
    shareable_session_token: Option<String>,
    event: &'a Event,
}

impl CalendarLinkOverviewTemplate<'_> {
    fn ical_link(&self) -> Result<String, AppError> {
        self.generic_calendar_link("ical")
    }

    fn frab_xml_link(&self) -> Result<String, AppError> {
        self.generic_calendar_link("frab_xml")
    }

    fn generic_calendar_link(&self, endpoint_name: &str) -> Result<String, AppError> {
        let mut url = self
            .base
            .request
            .url_for(endpoint_name, &[self.event.id.to_string()])?;
        url.set_query(Some(&serde_urlencoded::to_string(
            crate::web::ical::ICalQueryParams::with_session_token(
                self.shareable_session_token
                    .as_ref()
                    .ok_or(AppError::InternalError(
                        "Kein Shareable Session Token wurde gefunden.".to_owned(),
                    ))?
                    .clone(),
            ),
        )?));
        Ok(url.to_string())
    }

    fn login_url(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut url = self
            .base
            .request
            .url_for("login_form", [&self.event.id.to_string()])?;
        url.set_query(Some(&serde_urlencoded::to_string(
            super::auth::LoginQueryData {
                privilege: None,
                redirect_to: Some(self.base.request.full_url().to_string()),
            },
        )?));
        Ok(url.to_string())
    }
}
