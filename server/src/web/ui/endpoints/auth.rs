use crate::auth_session::SessionToken;
use crate::data_store::auth_token::Privilege;
use crate::data_store::StoreError;
use crate::web::ui::base_template::{AnyEventData, BaseTemplateContext};
use crate::web::ui::error::AppError;
use crate::web::ui::flash::{FlashMessage, FlashType, FlashesInterface};
use crate::web::ui::util;
use crate::web::ui::util::{SESSION_COOKIE_MAX_AGE, SESSION_COOKIE_NAME};
use crate::web::{time_calculation, AppState};
use actix_web::http::header;
use actix_web::http::header::{ContentType, TryIntoHeaderValue};
use actix_web::web::{Html, Query};
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use askama::Template;
use log::warn;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct LoginQueryData {
    pub redirect_to: Option<String>,
    pub privilege: Option<Privilege>,
}

#[get("/{event_id}/login")]
async fn login_form(
    path: web::Path<i32>,
    req: HttpRequest,
    state: web::Data<AppState>,
    query_data: Query<LoginQueryData>,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let query_data = query_data.into_inner();

    let session_token =
        util::extract_session_token_if_present(&state, &req, Privilege::ShowKueaPlan, event_id)?;
    let (event, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = session_token
            .map(|token| store.get_auth_token_for_session(&token, event_id))
            .transpose()?;
        Ok((store.get_event(event_id)?, auth))
    })
    .await??;

    let mut form_submit_url = req.url_for("login", &[event_id.to_string()])?;
    form_submit_url.set_query(Some(&serde_urlencoded::to_string(&query_data)?));

    let tmpl = LoginFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Login",
            event: AnyEventData::BasicEvent(&event),
            current_date: None,
            auth_token: auth.as_ref(),
            active_main_nav_button: None,
        },
        login_url: form_submit_url,
        expected_privilege: query_data.privilege.unwrap_or(Privilege::ShowKueaPlan),
    };
    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/login")]
async fn login(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    data: web::Form<LoginFormData>,
    query_data: Query<LoginQueryData>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let mut session_token =
        util::extract_session_token_if_present(&state, &req, Privilege::ShowKueaPlan, event_id)?
            .unwrap_or(SessionToken::new());

    let store = state.store.clone();
    let expected_privilege = query_data.privilege;
    let (
        event,
        extended_event,
        login_success,
        passphrase_expired,
        privilege_unlocked,
        session_token,
        auth,
    ) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let event = store.get_event(event_id)?;
        let (login_result, passphrase_expired) = match store.authenticate_with_passphrase(
            event_id,
            &data.passphrase,
            &mut session_token,
        ) {
            Ok(()) => (true, false),
            Err(StoreError::NotExisting) => (false, false),
            Err(StoreError::NotValid) => (false, true),
            Err(e) => return Err(e.into()),
        };
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        let extended_event = if auth.has_privilege(event_id, Privilege::ShowKueaPlan) {
            Some(store.get_extended_event(&auth, event_id)?)
        } else {
            None
        };
        Ok((
            event,
            extended_event,
            login_result,
            passphrase_expired,
            auth.has_privilege(
                event_id,
                expected_privilege.unwrap_or(Privilege::ShowKueaPlan),
            ),
            session_token,
            auth,
        ))
    })
    .await??;
    if !login_success {
        warn!(
            "HTTP 422 authentication failed. Client: <{}>{}",
            req.connection_info()
                .realip_remote_addr()
                .unwrap_or("unknown"),
            if passphrase_expired {
                ". Passphrase is not yet valid or has expired."
            } else {
                ""
            }
        );
    }

    if !login_success || !privilege_unlocked {
        let error = if !login_success {
            if passphrase_expired {
                "Die Passphrase ist nicht mehr (oder noch nicht) gültig."
            } else {
                "Ungültige Passphrase."
            }
        } else {
            "Diese Passphrase schaltet nicht den gewünschten Zugriff frei."
        };
        req.add_flash_message(FlashMessage {
            flash_type: FlashType::Error,
            message: error.to_string(),
            keep_open: true,
            button: None,
        });
        let mut form_submit_url = req.url_for("login", &[event_id.to_string()])?;
        form_submit_url.set_query(Some(&serde_urlencoded::to_string(query_data.into_inner())?));
        let tmpl = LoginFormTemplate {
            base: BaseTemplateContext {
                request: &req,
                page_title: "Login",
                event: if let Some(e) = extended_event.as_ref() {
                    AnyEventData::ExtendedEvent(e)
                } else {
                    AnyEventData::BasicEvent(&event)
                },
                current_date: None,
                auth_token: Some(&auth),
                active_main_nav_button: None,
            },
            login_url: form_submit_url,
            expected_privilege: expected_privilege.unwrap_or(Privilege::ShowKueaPlan),
        };

        let mut response = HttpResponse::UnprocessableEntity();
        response.cookie(create_session_cookie(session_token, &state.secret));
        Ok(response
            .append_header((
                header::CONTENT_TYPE,
                ContentType::html().try_into_value().unwrap(),
            ))
            .body(tmpl.render()?))
    } else {
        let mut response = HttpResponse::SeeOther();
        response.cookie(create_session_cookie(session_token, &state.secret));
        req.add_flash_message(FlashMessage {
            flash_type: FlashType::Success,
            message: "Login erfolgreich".to_owned(),
            keep_open: false,
            button: None,
        });
        Ok(response
            .append_header((
                header::LOCATION,
                if let Some(ref redirect_to) = query_data.redirect_to {
                    redirect_to.clone()
                } else if let Some(e) = extended_event.as_ref() {
                    req.url_for(
                        "main_list",
                        &[
                            event_id.to_string(),
                            time_calculation::most_reasonable_date(e).to_string(),
                        ],
                    )?
                    .to_string()
                } else {
                    req.url_for("event_index", &[event_id.to_string()])?
                        .to_string()
                },
            ))
            .finish())
    }
}

pub fn create_session_cookie<'b>(
    session_token: SessionToken,
    secret: &str,
) -> actix_web::cookie::Cookie<'b> {
    let mut cookie =
        actix_web::cookie::Cookie::new(SESSION_COOKIE_NAME, session_token.as_string(secret));
    cookie.set_path("/");
    cookie.set_expires(actix_web::cookie::time::OffsetDateTime::now_utc() + SESSION_COOKIE_MAX_AGE);
    cookie
}

#[derive(Template)]
#[template(path = "login_form.html")]
struct LoginFormTemplate<'a> {
    base: BaseTemplateContext<'a>,
    login_url: url::Url,
    expected_privilege: Privilege,
}

#[derive(Deserialize)]
struct LoginFormData {
    passphrase: String,
}

#[derive(Deserialize, Serialize)]
pub struct LogoutQueryData {
    pub redirect_to: Option<String>,
}

#[post("/logout")]
async fn logout_all(
    req: HttpRequest,
    query_data: Query<LogoutQueryData>,
) -> Result<impl Responder, AppError> {
    let mut response = HttpResponse::SeeOther();
    let mut cookie = actix_web::cookie::Cookie::new(SESSION_COOKIE_NAME, "");
    cookie.set_path("/");
    cookie.make_removal();
    response.cookie(cookie);
    req.add_flash_message(FlashMessage {
        flash_type: FlashType::Success,
        message: "Login-Daten wurden bereinigt.".to_owned(),
        keep_open: false,
        button: None,
    });
    Ok(response
        .append_header((
            header::LOCATION,
            if let Some(ref redirect_to) = query_data.redirect_to {
                redirect_to.clone()
            } else {
                req.url_for_static("events_list")?.to_string()
            },
        ))
        .finish())
}
