use actix_web::error::UrlGenerationError;
use actix_web::http::StatusCode;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder, ResponseError};
use rinja::Template;
use rust_embed::Embed;
use std::fmt::{Display, Formatter};

mod endpoints;

pub fn configure_app(cfg: &mut web::ServiceConfig) {
    cfg.service(get_ui_service());
}

fn get_ui_service() -> actix_web::Scope {
    web::scope("/ui")
        .service(static_resources)
        .service(endpoints::main_list)
        .default_service(web::to(not_found_handler))
}

#[derive(Embed)]
#[folder = "static/"]
struct Resources;

impl Resources {
    fn handle_embedded_file(path: &str) -> HttpResponse {
        match Self::get(path) {
            Some(content) => HttpResponse::Ok()
                .content_type(mime_guess::from_path(path).first_or_octet_stream().as_ref())
                .body(content.data.into_owned()),
            None => {
                HttpResponse::NotFound().body(format!("Static resource file '{}' not found", path))
            }
        }
    }
}

#[get("/static/{_:.*}")]
async fn static_resources(path: web::Path<String>) -> impl Responder {
    Resources::handle_embedded_file(path.as_str())
}

#[derive(Debug)]
struct BaseTemplateContext<'a> {
    request: &'a HttpRequest,
    page_title: &'a str,
}

impl BaseTemplateContext<'_> {
    fn url_for_static(&self, file: &str) -> Result<String, UrlGenerationError> {
        Ok(self
            .request
            .url_for("static_resources", &[file])?
            .to_string())
    }
}

#[derive(Debug)]
enum AppError {
    NotFound,
    TemplateError(rinja::Error),
    UrlError(UrlGenerationError),
}

impl From<rinja::Error> for AppError {
    fn from(value: rinja::Error) -> Self {
        AppError::TemplateError(value)
    }
}

impl From<UrlGenerationError> for AppError {
    fn from(value: UrlGenerationError) -> Self {
        AppError::UrlError(value)
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NotFound => write!(f, "Not found"),
            AppError::TemplateError(e) => write!(f, "Error rendering template: {}", e),
            AppError::UrlError(e) => write!(f, "Could not generate url: {}", e),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl Responder for AppError {
    type Body = String;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        // The error handler uses a rinja template to display its content.
        // The member `lang` is used by "_layout.html" which "error.html" extends. Even though it
        // is always the fallback language English in here, "_layout.html" expects to be able to
        // access this field, so you have to provide it.
        #[derive(Debug, Template)]
        #[template(path = "error.html")]
        struct ErrorTemplate<'a> {
            base: BaseTemplateContext<'a>,
            error: &'a AppError,
        }

        let tmpl = ErrorTemplate {
            base: BaseTemplateContext {
                request: &req,
                page_title: "Error",
            },
            error: &self,
        };
        if let Ok(body) = tmpl.render() {
            (Html::new(body), self.status_code()).respond_to(req)
        } else {
            ("Something went wrong".to_string(), self.status_code()).respond_to(req)
        }
    }
}

async fn not_found_handler() -> AppError {
    AppError::NotFound
}
