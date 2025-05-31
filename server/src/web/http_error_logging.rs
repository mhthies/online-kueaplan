use crate::web::api::APIError;
use crate::web::ui::error::AppError;
use log::{error, warn};

pub async fn error_logging_middleware<B: actix_web::body::MessageBody>(
    req: actix_web::dev::ServiceRequest,
    next: actix_web::middleware::Next<B>,
) -> Result<actix_web::dev::ServiceResponse<B>, actix_web::Error> {
    let response = next.call(req).await?;

    if let Some(error) = response.response().error() {
        if let Some(app_error) = error.as_error::<AppError>() {
            match app_error {
                AppError::PageNotFound => {
                    warn!(
                        "HTTP {} page not found at <{}>",
                        response.response().status(),
                        response.request().uri()
                    );
                }
                AppError::InvalidData(e) => {
                    warn!(
                        "HTTP {} invalid data at <{}>: {}",
                        response.response().status(),
                        response.request().uri(),
                        e
                    );
                }
                AppError::PermissionDenied {
                    required_privilege,
                    event_id: _,
                    session_error,
                } => {
                    if let Some(session_error) = session_error {
                        warn!(
                            "HTTP {} invalid session token. Client: <{}> Cause: {:?}",
                            response.response().status(),
                            response
                                .request()
                                .peer_addr()
                                .map(|a| a.to_string())
                                .unwrap_or("unknown".to_owned()),
                            session_error
                        );
                    } else {
                        warn!(
                            "HTTP {} permission denied at <{}>. Client: <{}> Requires privilege: {:?}",
                            response.response().status(),
                            response.request().uri(),
                            response.request().peer_addr().map(|a| a.to_string()).unwrap_or("unknown".to_owned()),
                            required_privilege
                        );
                    }
                }
                AppError::EntityNotFound
                | AppError::ConcurrentEditConflict
                | AppError::TransactionConflict => {}
                AppError::DatabaseConnectionError(e) => {
                    error!(
                        "HTTP {} database connection error: {}",
                        response.response().status(),
                        e
                    );
                }
                AppError::InternalError(e) => {
                    error!(
                        "HTTP {} internal server error at <{}>: {}",
                        response.response().status(),
                        response.request().uri(),
                        e
                    );
                }
            }
        } else if let Some(api_error) = error.as_error::<APIError>() {
            match api_error {
                APIError::PermissionDenied { required_privilege } => {
                    warn!(
                        "HTTP {} permission denied at <{}>. Client: <{}> Requires privilege: {:?}",
                        response.response().status(),
                        response.request().uri(),
                        response
                            .request()
                            .peer_addr()
                            .map(|a| a.to_string())
                            .unwrap_or("unknown".to_owned()),
                        required_privilege
                    );
                }
                APIError::NoSessionToken => {
                    warn!(
                        "HTTP {} permission denied at <{}>. Client: <{}> Cause: No session token",
                        response.response().status(),
                        response.request().uri(),
                        response
                            .request()
                            .peer_addr()
                            .map(|a| a.to_string())
                            .unwrap_or("unknown".to_owned()),
                    );
                }
                APIError::InvalidSessionToken => {
                    warn!(
                        "HTTP {} invalid session token. Client: <{}>",
                        response.response().status(),
                        response
                            .request()
                            .peer_addr()
                            .map(|a| a.to_string())
                            .unwrap_or("unknown".to_owned()),
                    );
                }
                APIError::AuthenticationFailed => {
                    warn!(
                        "HTTP {} authentication failed. Client: <{}>",
                        response.response().status(),
                        response
                            .request()
                            .peer_addr()
                            .map(|a| a.to_string())
                            .unwrap_or("unknown".to_owned()),
                    );
                }
                APIError::NotExisting
                | APIError::AlreadyExisting
                | APIError::InvalidJson(_)
                | APIError::InvalidData(_)
                | APIError::EntityIdMissmatch
                | APIError::TransactionConflict
                | APIError::ConcurrentEditConflict => {}
                APIError::InternalError(e) => {
                    error!(
                        "HTTP {} internal server error at <{}>: {}",
                        response.response().status(),
                        response.request().uri(),
                        e
                    );
                }
            }
            error!("{:?}", api_error);
        } else {
            error!(
                "HTTP {} unexpected error at <{}>: {:?}",
                response.response().status(),
                response.request().uri(),
                error
            );
        }
    }
    Ok(response)
}
