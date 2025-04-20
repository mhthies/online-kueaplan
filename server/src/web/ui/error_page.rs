//! This module provides functionality to generate nice-looking error pages for errors returned from
//! handler functions.
//!
//! This is achieved by an actix-web middleware that replaces the original HTTP response in the case
//! of an error. In contrast to rendering the error page in our [actix_web::ResponseError]
//! implementation, this allows us to access the HTTP Request, e.g. for generating URLs to static
//! files and other pages.
use crate::data_store::auth_token::Privilege;
use crate::data_store::EventId;
use crate::web::ui::base_template::BaseTemplateContext;
use crate::web::ui::error::AppError;
use actix_web::body::EitherBody;
use actix_web::web::Html;
use actix_web::{HttpRequest, HttpResponse, Responder, ResponseError};
use askama::Template;

/// An actix-web middleware for generating nice error pages
///
/// The middleware replaces the existing HTTP response (typically generated from the error's
/// ResponseError implementation) with a nice error page, when an error has been returned by the
/// endpoint handler function. The nice error page is generated from askama templates, extending the
/// "base.html" template to keep the application's look & feel. In case, rendering the template
/// fails (maybe, something is really borked in the application), we fall back to a plain text
/// representation of the error.
///
/// If the error, returned from the endpoint handler, is an [AppError], we try to use the semantic
/// information to provide a descriptive and helpful error page to the user. Otherwise, we simply
/// show the string representation of the error.
///
/// Typical usage:
/// ```ignore
/// use crate::web::ui::error::AppError;
///
/// let service = actix_web::web::scope("/ui")
///     .service(login_form)
///     .wrap(actix_web::middleware::from_fn(error_page_middleware));
///
/// #[actix_web::get("/my_endpoint")]
/// async fn login_form() -> Result<impl actix_web::Responder, AppError> {
///     todo!()
/// }
/// ```
pub async fn error_page_middleware<B: actix_web::body::MessageBody>(
    req: actix_web::dev::ServiceRequest,
    next: actix_web::middleware::Next<B>,
) -> Result<actix_web::dev::ServiceResponse<EitherBody<B, String>>, actix_web::Error> {
    let response = next.call(req).await?;

    if response.response().error().is_some() {
        let (req, res) = response.into_parts();
        let error = res
            .error()
            .expect("We checked that res has an error, above.");
        if let Some(app_error) = error.as_error::<AppError>() {
            let response = generate_app_error_page(app_error, &req);
            Ok(actix_web::dev::ServiceResponse::new(
                req,
                response.map_body(|_, body| EitherBody::right(body)),
            ))
        } else {
            let response = generate_generic_error_page(error.as_response_error(), &req);
            Ok(actix_web::dev::ServiceResponse::new(
                req,
                response.map_body(|_, body| EitherBody::right(body)),
            ))
        }
    } else {
        Ok(response.map_body(|_, body| EitherBody::left(body)))
    }
}

/// Generate a nice error page with additional information and help for the given [AppError].
fn generate_app_error_page(
    app_error: &AppError,
    http_request: &HttpRequest,
) -> HttpResponse<String> {
    let tmpl = AppErrorTemplate {
        base: BaseTemplateContext {
            request: http_request,
            page_title: "Fehler",
        },
        error: app_error,
        url: &http_request.full_url(),
        timestamp: chrono::Local::now(),
        admin_name: "TODO",
        admin_email: "mail@example.com",
    };
    render_template_or_show_error_as_string(tmpl, app_error, http_request)
}

/// Generate a nice error page for the given `error`, using its string representation.
fn generate_generic_error_page(
    error: &dyn ResponseError,
    http_request: &HttpRequest,
) -> HttpResponse<String> {
    let tmpl = ErrorTemplate {
        base: BaseTemplateContext {
            request: http_request,
            page_title: "Fehler",
        },
        error,
        url: &http_request.full_url(),
        timestamp: chrono::Local::now(),
        admin_name: "TODO",
        admin_email: "mail@example.com",
    };
    render_template_or_show_error_as_string(tmpl, error, http_request)
}

/// Try to render the given [askama::Template] structure and generate an HTTP response as an HTML
/// error page for the given error and create an HTTP response.
///
/// In case of an error while rendering the template, return a plain text HTTP response with the
/// error's string representation.
fn render_template_or_show_error_as_string(
    tmpl: impl Template,
    error: &dyn ResponseError,
    req: &HttpRequest,
) -> HttpResponse<String> {
    match tmpl.render() {
        Ok(body) => (Html::new(body), error.status_code()).respond_to(req),
        Err(err) => (
            format!(
                "Error: {}\n(Could not render nice error page: {})",
                error, err
            ),
            error.status_code(),
        )
            .respond_to(req),
    }
}

#[derive(Debug, Template)]
#[template(path = "app_error.html")]
struct AppErrorTemplate<'a> {
    base: BaseTemplateContext<'a>,
    error: &'a AppError,
    url: &'a url::Url,
    timestamp: chrono::DateTime<chrono::Local>,
    admin_name: &'a str,
    admin_email: &'a str,
}

#[derive(Debug, Template)]
#[template(path = "error.html")]
struct ErrorTemplate<'a> {
    base: BaseTemplateContext<'a>,
    error: &'a dyn ResponseError,
    url: &'a url::Url,
    timestamp: chrono::DateTime<chrono::Local>,
    admin_name: &'a str,
    admin_email: &'a str,
}

impl AppErrorTemplate<'_> {
    fn login_url_for(
        &self,
        redirect_url: &url::Url,
        privilege: Privilege,
        event_id: EventId,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut url = self
            .base
            .request
            .url_for("login_form", [&event_id.to_string()])?;
        url.set_query(Some(&serde_urlencoded::to_string(
            super::endpoints::auth::LoginQueryData {
                privilege: Some(privilege),
                redirect_to: Some(redirect_url.to_string()),
            },
        )?));
        Ok(url.to_string())
    }
}
