use crate::auth_session::{SessionError, SessionToken};
use crate::data_store::auth_token::Privilege;
use crate::web::ui::base_template::BaseTemplateContext;
use crate::web::ui::error::AppError;
use crate::web::ui::flash::{FlashMessage, FlashType, FlashesInterface};
use crate::web::ui::time_calculation;
use crate::web::ui::util::{SESSION_COOKIE_MAX_AGE, SESSION_COOKIE_NAME};
use crate::web::AppState;
use actix_web::http::header;
use actix_web::http::header::{ContentType, TryIntoHeaderValue};
use actix_web::web::{Html, Query};
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use askama::Template;
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
    query_data: Query<LoginQueryData>,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let expected_privilege = query_data.privilege;

    let mut form_submit_url = req.url_for("login", &[event_id.to_string()])?;
    form_submit_url.set_query(Some(&serde_urlencoded::to_string(query_data.into_inner())?));

    // TODO add event name; 404 if event does not exist
    let tmpl = LoginFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Login",
        },
        login_url: form_submit_url,
        expected_privilege: expected_privilege.unwrap_or(Privilege::ShowKueaPlan),
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

    let mut session_token = req
        .cookie(SESSION_COOKIE_NAME)
        .map(|cookie| {
            SessionToken::from_string(cookie.value(), &state.secret, SESSION_COOKIE_MAX_AGE)
        })
        .unwrap_or(Err(SessionError::InvalidTokenStructure))
        .unwrap_or(SessionToken::new());

    let store = state.store.clone();
    let event = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&SessionToken::new(), event_id)?;
        let event = store.get_event(&auth, event_id)?;
        Ok(event)
    })
    .await??;
    let store = state.store.clone();
    let expected_privilege = query_data.privilege;
    let result = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        store.authenticate_with_passphrase(event_id, &data.passphrase, &mut session_token)?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok((
            session_token,
            auth.has_privilege(
                event_id,
                expected_privilege.unwrap_or(Privilege::ShowKueaPlan),
            ),
        ))
    })
    .await?;

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
        req.add_flash_message(FlashMessage {
            flash_type: FlashType::Error,
            message: error.to_string(),
            keep_open: true,
        });
        let mut form_submit_url = req.url_for("login", &[event_id.to_string()])?;
        form_submit_url.set_query(Some(&serde_urlencoded::to_string(query_data.into_inner())?));
        let tmpl = LoginFormTemplate {
            base: BaseTemplateContext {
                request: &req,
                page_title: "Login",
            },
            login_url: form_submit_url,
            expected_privilege: expected_privilege.unwrap_or(Privilege::ShowKueaPlan),
        };

        let mut response = HttpResponse::UnprocessableEntity();
        if let Some(session_token) = session_token {
            response.cookie(create_session_cookie(session_token, &state.secret));
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
            response.cookie(create_session_cookie(session_token, &state.secret));
        }
        req.add_flash_message(FlashMessage {
            flash_type: FlashType::Success,
            message: "Login erfolgreich".to_owned(),
            keep_open: false,
        });
        Ok(response
            .append_header((
                header::LOCATION,
                if let Some(ref redirect_to) = query_data.redirect_to {
                    redirect_to.clone()
                } else {
                    req.url_for(
                        "main_list",
                        &[
                            event_id.to_string(),
                            time_calculation::most_reasonable_date(event).to_string(),
                        ],
                    )?
                    .to_string()
                },
            ))
            .finish())
    }
}

fn create_session_cookie(session_token: SessionToken, secret: &str) -> actix_web::cookie::Cookie {
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
