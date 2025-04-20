use crate::auth_session::SessionError;
use crate::data_store::auth_token::Privilege;
use crate::data_store::{EventId, StoreError};
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
/// [crate::web::ui::error_page::error_page_middleware] middleware, because actix-web's
/// ResponseError trait is quite restricted in what it can do.
#[derive(Debug, Clone)]
pub enum AppError {
    PageNotFound,
    EntityNotFound,
    PermissionDenied {
        required_privilege: Privilege,
        event_id: EventId,
        session_error: Option<SessionError>,
    },
    ConcurrentEditConflict,
    TransactionConflict,
    DatabaseConnectionError(String),
    InternalError(String),
}

impl std::error::Error for AppError {}

impl From<StoreError> for AppError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::ConnectionError(error) => Self::DatabaseConnectionError(error),
            StoreError::QueryError(diesel_error) => {
                Self::InternalError(format!("Database query failed: {}", diesel_error))
            }
            StoreError::TransactionConflict => Self::TransactionConflict,
            StoreError::NotExisting => Self::EntityNotFound,
            StoreError::ConflictEntityExists => {
                Self::InternalError("Conflicting entity exists".to_owned())
            }
            StoreError::ConcurrentEditConflict => Self::ConcurrentEditConflict,
            StoreError::PermissionDenied {
                required_privilege,
                event_id: Some(event_id),
            } => Self::PermissionDenied {
                required_privilege,
                event_id,
                session_error: None,
            },
            StoreError::PermissionDenied {
                required_privilege,
                event_id: None,
            } => Self::InternalError(format!(
                "Global privilege {:?} required.",
                required_privilege
            )),
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
        if let askama::Error::Custom(error) = value {
            match error.downcast::<AppError>() {
                Ok(e) => {
                    return *e;
                }
                Err(error) => match error.downcast::<StoreError>() {
                    Ok(e) => {
                        return (*e).into();
                    }
                    Err(error) => {
                        return AppError::InternalError(format!("{:?}", error));
                    }
                },
            }
        }
        AppError::InternalError(format!("Error while rendering template: {}", value))
    }
}

impl From<UrlGenerationError> for AppError {
    fn from(value: UrlGenerationError) -> Self {
        AppError::InternalError(format!("Could not generate URL: {}", value))
    }
}

impl From<serde_urlencoded::ser::Error> for AppError {
    fn from(value: serde_urlencoded::ser::Error) -> Self {
        AppError::InternalError(format!(
            "Error while serializing URL query parameters: {}",
            value
        ))
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::PageNotFound => write!(f, "Not found"),
            AppError::TransactionConflict => {
                write!(f, "Concurrent database transaction conflict. Please retry.")
            }
            AppError::EntityNotFound => write!(f, "Entity not found"),
            AppError::PermissionDenied {
                required_privilege,
                event_id: _,
                session_error,
            } => {
                write!(
                    f,
                    "Client is not authorized to perform this action. Authentication as {} is required",
                    required_privilege
                        .qualifying_roles()
                        .iter()
                        .map(|role| role.name().to_owned())
                        .collect::<Vec<String>>()
                        .join(" or ")
                )?;
                if let Some(session_error) = session_error {
                    write!(
                        f,
                        " Session was present, but invalid, because of {:?}",
                        session_error
                    )?;
                }
                Ok(())
            }
            AppError::ConcurrentEditConflict => {
                f.write_str("Editing entity refused due to a concurrent update of the entity.")
            }
            AppError::DatabaseConnectionError(e) => {
                write!(f, "Could not connect to database: {}", e)
            }
            AppError::InternalError(e) => write!(f, "Internal program error: {}", e),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::PageNotFound | AppError::EntityNotFound => StatusCode::NOT_FOUND,
            AppError::PermissionDenied { .. } => StatusCode::FORBIDDEN,
            AppError::TransactionConflict => StatusCode::SERVICE_UNAVAILABLE,
            AppError::ConcurrentEditConflict => StatusCode::CONFLICT,
            AppError::DatabaseConnectionError(_) | AppError::InternalError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}
