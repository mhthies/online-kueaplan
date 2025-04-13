use crate::auth_session::SessionError;
use crate::data_store::auth_token::Privilege;
use crate::data_store::StoreError;
use actix_web::error::UrlGenerationError;
use actix_web::http::StatusCode;
use actix_web::ResponseError;
use std::fmt::{Display, Formatter};

/// Semantic error type for ui endpoint functions
///
/// The different enum items are meant to produce different descriptive and helpful error pages for
/// the user, with an appropriate HTTP status code.
///
/// The error pages are generated using the
/// [crate::web::ui::framework::error_page::error_page_middleware] middleware, because actix-web's
/// ResponseError trait is quite restricted in what it can do.
#[derive(Debug)]
pub enum AppError {
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
