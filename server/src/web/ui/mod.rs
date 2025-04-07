use crate::auth_session::SessionError;
use crate::data_store::auth_token::Privilege;
use crate::data_store::StoreError;
use crate::web::ui::framework::error_page::error_page_middleware;
use crate::web::ui::framework::flash::flash_middleware;
use actix_web::error::UrlGenerationError;
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::http::StatusCode;
use actix_web::middleware::from_fn;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder, ResponseError};
use framework::flash::FlashesInterface;
use rust_embed::Embed;
use std::fmt::{Display, Formatter};

mod auth;
mod edit_entry;
mod framework;
mod main_list;
mod util;

#[allow(clippy::identity_op)] // We want to explicitly state that it's "1" year
const SESSION_COOKIE_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(1 * 86400 * 365);

pub fn configure_app(cfg: &mut web::ServiceConfig) {
    cfg.service(
        get_ui_service()
            .wrap(from_fn(flash_middleware))
            .wrap(from_fn(error_page_middleware)),
    );
}

fn get_ui_service() -> actix_web::Scope {
    web::scope("/ui")
        .service(static_resources)
        .service(main_list::main_list)
        .service(auth::login_form)
        .service(auth::login)
        .service(edit_entry::edit_entry_form)
        .service(edit_entry::edit_entry)
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
                .append_header(CacheControl(vec![CacheDirective::MaxAge(86400 * 365)]))
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
        let mut url = self.request.url_for("static_resources", &[file])?;
        url.query_pairs_mut().append_pair(
            "hash",
            &Resources::get(file)
                .map(|f| bytes_to_hex(&f.metadata.sha256_hash()))
                .unwrap_or("unknown".to_string()),
        );
        Ok(url.to_string())
    }

    fn get_flashes(&self) -> Vec<framework::flash::FlashMessage> {
        self.request.get_and_clear_flashes()
    }
}

#[derive(Debug)]
enum AppError {
    PageNotFound,
    EntityNotFound,
    NoSession,
    InvalidSessionToken,
    ExpiredSessionToken,
    PermissionDenied { required_privilege: Privilege },
    TemplateError(askama::Error),
    UrlError(UrlGenerationError),
    TransactionConflict,
    InternalError(String),
}

impl From<StoreError> for AppError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::ConnectionError(error) => {
                Self::InternalError(format!("Could not connect to database: {}", error))
            }
            StoreError::QueryError(diesel_error) => Self::InternalError(format!(
                "Error while executing database query: {}",
                diesel_error
            )),
            StoreError::TransactionConflict => Self::TransactionConflict,
            StoreError::NotExisting => Self::EntityNotFound,
            StoreError::ConflictEntityExists => {
                Self::InternalError("Conflicting entity exists".to_owned())
            }
            StoreError::PermissionDenied { required_privilege } => {
                Self::PermissionDenied { required_privilege }
            }
            StoreError::InvalidInputData(e) => Self::InternalError(format!("Invalid data: {}", e)),
            StoreError::InvalidDataInDatabase(e) => Self::InternalError(format!(
                "Data queried from database could not be deserialized: {}",
                e
            )),
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

impl From<askama::Error> for AppError {
    fn from(value: askama::Error) -> Self {
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
            AppError::TransactionConflict => {
                write!(f, "Concurrent database transaction conflict. Please retry.")
            }
            AppError::EntityNotFound => write!(f, "Entity not found"),
            AppError::PermissionDenied { required_privilege } => write!(
                f,
                "Client is not authorized to perform this action. Authentication as {} is required",
                required_privilege
                    .qualifying_roles()
                    .iter()
                    .map(|role| role.name().to_owned())
                    .collect::<Vec<String>>()
                    .join(" or ")
            ),
            AppError::InternalError(e) => write!(f, "Internal error: {}", e),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::PageNotFound | AppError::EntityNotFound => StatusCode::NOT_FOUND,
            AppError::NoSession
            | AppError::InvalidSessionToken
            | AppError::ExpiredSessionToken
            | AppError::PermissionDenied {
                required_privilege: _,
            } => StatusCode::FORBIDDEN,
            AppError::TransactionConflict => StatusCode::SERVICE_UNAVAILABLE,
            AppError::TemplateError(_) | AppError::UrlError(_) | AppError::InternalError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

async fn not_found_handler() -> Result<&'static str, AppError> {
    Err(AppError::PageNotFound)
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
