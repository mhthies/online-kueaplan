use crate::auth_session::{SessionError, SessionToken};
use crate::data_store::AccessRole;
use crate::web::ui::{AppError, BaseTemplateContext};
use crate::web::AppState;
use actix_web::cookie::Cookie;
use actix_web::http::header;
use actix_web::http::header::{ContentType, TryIntoHeaderValue};
use actix_web::web::Html;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use rinja::Template;
use serde::Deserialize;

#[get("/{event_id}/login")]
async fn login_form(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();

    // TODO add event name; 404 if event does not exist
    let tmpl = LoginFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Login",
        },
        login_url: req.url_for("login", &[event_id.to_string()])?,
        error: None,
    };
    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/login")]
async fn login(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    data: web::Form<LoginFormData>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();

    let mut session_token = req
        .cookie("kuea-plan-session")
        .map(|cookie| {
            SessionToken::from_string(cookie.value(), &state.secret, super::SESSION_COOKIE_MAX_AGE)
        })
        .unwrap_or(Err(SessionError::InvalidTokenStructure))
        .unwrap_or(SessionToken::new());

    let store = state.store.clone();
    let result = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        store.authorize(event_id, &data.passphrase, &mut session_token)?;
        let auth = store.check_authorization(&session_token, event_id)?;
        Ok((session_token, auth.has_privilege(AccessRole::User)))
    })
    .await?; // TODO handle authorization errors by showing form again

    let (session_token, error) = match result {
        Ok((session_token, true)) => (Some(session_token), None),
        Ok((session_token, false)) => (
            Some(session_token),
            Some("Diese Passphrase schaltet nicht den gewünschten Zugriff frei."),
        ),
        Err(AppError::EntityNotFound) => (None, Some("Ungültige Passphrase.")),
        Err(e) => return Err(e),
    };

    if let Some(error) = error {
        let tmpl = LoginFormTemplate {
            base: BaseTemplateContext {
                request: &req,
                page_title: "Login",
            },
            login_url: req.url_for("login", &[event_id.to_string()])?,
            error: Some(error),
        };

        let mut response = HttpResponse::UnprocessableEntity();
        if let Some(session_token) = session_token {
            response.cookie(Cookie::new(
                "kuea-plan-session",
                session_token.as_string(&state.secret),
            ));
        }
        Ok(response
            .append_header((
                header::CONTENT_TYPE,
                ContentType::html().try_into_value().unwrap(),
            ))
            .body(tmpl.render()?))
    } else {
        let mut response = HttpResponse::SeeOther();
        if let Some(session_token) = session_token {
            response.cookie(Cookie::new(
                "kuea-plan-session",
                session_token.as_string(&state.secret),
            ));
        }
        Ok(response
            .append_header((
                header::LOCATION,
                req.url_for("main_list", &[event_id.to_string()])?
                    .to_string(),
            ))
            .finish())
    }
}

#[derive(Template)]
#[template(path = "login_form.html")]
struct LoginFormTemplate<'a> {
    base: BaseTemplateContext<'a>,
    login_url: url::Url,
    error: Option<&'a str>,
}

#[derive(Deserialize)]
struct LoginFormData {
    passphrase: String,
}
