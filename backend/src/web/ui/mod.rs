use crate::auth_session::SessionError;
use crate::data_store::StoreError;
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
    PageNotFound,
    EntityNotFound,
    NoSession,
    InvalidSessionToken,
    ExpiredSessionToken,
    PermissionDenied,
    AuthenticationFailed,
    TemplateError(rinja::Error),
    UrlError(UrlGenerationError),
    BackendError(String),
    InternalError(String),
}

impl From<StoreError> for AppError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::ConnectionPoolError(r2d2_error) => Self::BackendError(format!(
                "Could not get database connection from pool: {}",
                r2d2_error
            )),
            StoreError::ConnectionError(diesel_error) => {
                Self::BackendError(diesel_error.to_string())
            }
            StoreError::QueryError(diesel_error) => Self::BackendError(diesel_error.to_string()),
            StoreError::NotExisting => Self::EntityNotFound,
            StoreError::ConflictEntityExists => {
                Self::InternalError("Conflicting entity exists".to_owned())
            }
            StoreError::PermissionDenied => Self::PermissionDenied,
            StoreError::InvalidData => Self::InternalError("Invalid data".to_owned()),
        }
    }
}

impl From<actix_web::error::BlockingError> for AppError {
    fn from(_e: actix_web::error::BlockingError) -> Self {
        AppError::InternalError(
            "Could not get thread from thread pool for synchronous database operation.".to_owned(),
        )
    }
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

impl From<SessionError> for AppError {
    fn from(value: SessionError) -> Self {
        match value {
            SessionError::ExpiredToken => AppError::ExpiredSessionToken,
            _ => AppError::InvalidSessionToken,
        }
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::PageNotFound => write!(f, "Not found"),
            AppError::TemplateError(e) => write!(f, "Error rendering template: {}", e),
            AppError::UrlError(e) => write!(f, "Could not generate url: {}", e),
            AppError::NoSession => write!(f, "Not authenticated"),
            AppError::InvalidSessionToken => write!(f, "Invalid session token"),
            AppError::ExpiredSessionToken => write!(f, "Session is expired"),
            AppError::EntityNotFound => write!(f, "Entity not found"),
            AppError::PermissionDenied => write!(f, "Permission denied"),
            AppError::AuthenticationFailed => write!(f, "Login failed"),
            AppError::BackendError(e) => write!(f, "Internal database error: {}", e),
            AppError::InternalError(e) => write!(f, "Internal error: {}", e),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::PageNotFound | AppError::EntityNotFound => StatusCode::NOT_FOUND,
            AppError::NoSession | AppError::InvalidSessionToken | AppError::ExpiredSessionToken => {
                StatusCode::FORBIDDEN
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

//
// impl Responder for AppError {
//     type Body = String;
//
//     fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
//         // The error handler uses a rinja template to display its content.
//         // The member `lang` is used by "_layout.html" which "error.html" extends. Even though it
//         // is always the fallback language English in here, "_layout.html" expects to be able to
//         // access this field, so you have to provide it.
//         #[derive(Debug, Template)]
//         #[template(path = "error.html")]
//         struct ErrorTemplate<'a> {
//             base: BaseTemplateContext<'a>,
//             error: &'a AppError,
//         }
//
//         let tmpl = ErrorTemplate {
//             base: BaseTemplateContext {
//                 request: &req,
//                 page_title: "Error",
//             },
//             error: &self,
//         };
//         if let Ok(body) = tmpl.render() {
//             (Html::new(body), self.status_code()).respond_to(req)
//         } else {
//             ("Something went wrong".to_string(), self.status_code()).respond_to(req)
//         }
//     }
// }

async fn not_found_handler() -> Result<&'static str, AppError> {
    Err(AppError::PageNotFound)
}
