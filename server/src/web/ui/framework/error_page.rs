use crate::web::ui::error::AppError;
use crate::web::ui::BaseTemplateContext;
use actix_web::body::EitherBody;
use actix_web::web::Html;
use actix_web::{HttpRequest, HttpResponse, Responder, ResponseError};
use askama::Template;

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

fn generate_app_error_page(
    app_error: &AppError,
    http_request: &HttpRequest,
) -> HttpResponse<String> {
    let tmpl = AppErrorTemplate {
        base: BaseTemplateContext {
            request: &http_request,
            page_title: "Error",
        },
        error: app_error,
    };
    render_template_or_show_error_as_string(tmpl, app_error, http_request)
}

fn generate_generic_error_page(
    error: &dyn ResponseError,
    http_request: &HttpRequest,
) -> HttpResponse<String> {
    let tmpl = ErrorTemplate {
        base: BaseTemplateContext {
            request: &http_request,
            page_title: "Error",
        },
        error,
    };
    render_template_or_show_error_as_string(tmpl, error, http_request)
}

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
                error.to_string(),
                err.to_string()
            ),
            error.status_code(),
        )
            .respond_to(req),
    }
}

// TODO better template for App errors
#[derive(Debug, Template)]
#[template(path = "error.html")]
struct AppErrorTemplate<'a> {
    base: BaseTemplateContext<'a>,
    error: &'a AppError,
}

#[derive(Debug, Template)]
#[template(path = "error.html")]
struct ErrorTemplate<'a> {
    base: BaseTemplateContext<'a>,
    error: &'a dyn ResponseError,
}
