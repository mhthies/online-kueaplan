use crate::auth_session::{SessionError, SessionToken};
use crate::data_store::auth_token::AccessRole;
use crate::data_store::models::Event;
use crate::data_store::{EventFilter, EventId};
use crate::web::ui::base_template::{AnyEventData, BaseTemplateContext};
use crate::web::ui::endpoints::auth::create_session_cookie;
use crate::web::ui::error::AppError;
use crate::web::ui::flash::{FlashMessage, FlashType, FlashesInterface};
use crate::web::ui::util;
use crate::web::ui::util::format_access_role;
use crate::web::AppState;
use actix_web::http::header;
use actix_web::web::Html;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use askama::Template;
use serde::Deserialize;
use std::collections::BTreeMap;

#[get("/access_roles")]
async fn list_own_roles(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let session_token = req.cookie(util::SESSION_COOKIE_NAME).map(|cookie| {
        SessionToken::from_string(cookie.value(), &state.secret, util::SESSION_COOKIE_MAX_AGE)
    });
    let (session_token, session_error) = match session_token {
        None => (None, None),
        Some(Ok(token)) => (Some(token), None),
        Some(Err(error)) => (None, Some(error)),
    };

    let (events, roles) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let roles = if let Some(session_token) = session_token {
            store.list_all_access_roles(&session_token)?
        } else {
            vec![]
        };
        Ok((store.get_events(EventFilter::default())?, roles))
    })
    .await??;

    let access_roles_by_event: Vec<(EventId, Vec<AccessRole>)> =
        roles
            .into_iter()
            .fold(vec![], |mut accum, (event_id, role)| {
                match accum.last_mut() {
                    Some(current_entry) if current_entry.0 == event_id => {
                        current_entry.1.push(role);
                    }
                    _ => {
                        accum.push((event_id, vec![role]));
                    }
                };
                accum
            });

    let tmpl = ListOwnRolesTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Login-Status",
            event: AnyEventData::None,
            current_date: None,
            auth_token: None,
            active_main_nav_button: None,
        },
        access_roles_by_event: &access_roles_by_event,
        events: events.iter().map(|e| (e.id, e)).collect(),
        session_error: &session_error,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "list_own_roles.html")]
struct ListOwnRolesTemplate<'a> {
    base: BaseTemplateContext<'a>,
    access_roles_by_event: &'a Vec<(EventId, Vec<AccessRole>)>,
    events: BTreeMap<EventId, &'a Event>,
    session_error: &'a Option<SessionError>,
}

impl<'a> ListOwnRolesTemplate<'a> {
    fn get_event_title(&self, event_id: EventId) -> &str {
        self.events
            .get(&event_id)
            .map(|e| e.title.as_ref())
            .unwrap_or("???")
    }

    fn logout_all_url(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut url = self.base.request.url_for_static("logout_all")?;
        url.set_query(Some(&serde_urlencoded::to_string(
            super::auth::LogoutQueryData {
                redirect_to: Some(
                    self.base
                        .request
                        .url_for_static("list_own_roles")?
                        .to_string(),
                ),
            },
        )?));
        Ok(url.to_string())
    }
}

#[post("/logout_role")]
async fn logout_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    data: web::Form<LogoutRoleFormData>,
) -> Result<impl Responder, AppError> {
    let session_token = req
        .cookie(util::SESSION_COOKIE_NAME)
        .and_then(|cookie| {
            SessionToken::from_string(cookie.value(), &state.secret, util::SESSION_COOKIE_MAX_AGE)
                .ok()
        })
        .unwrap_or(SessionToken::new());
    let data = data.into_inner();

    let store = state.store.clone();
    let session_token = {
        web::block(move || -> Result<_, AppError> {
            let mut session_token = session_token;
            let mut store = store.get_facade()?;
            store.drop_access_role(data.event_id, data.role, &mut session_token)?;
            Ok(session_token)
        })
        .await??
    };

    let mut response = HttpResponse::SeeOther();
    response.cookie(create_session_cookie(session_token, &state.secret));
    req.add_flash_message(FlashMessage {
        flash_type: FlashType::Success,
        message: "Logout erfolgreich".to_owned(),
        keep_open: false,
        button: None,
    });
    Ok(response
        .append_header((
            header::LOCATION,
            req.url_for_static("list_own_roles")?.to_string(),
        ))
        .finish())
}

#[derive(Deserialize)]
struct LogoutRoleFormData {
    event_id: EventId,
    role: AccessRole,
}
