use std::fmt::Display;

mod endpoints_announcement;
mod endpoints_auth;
mod endpoints_category;
mod endpoints_entry;
mod endpoints_event;
mod endpoints_event_extended;
mod endpoints_passphrase;
mod endpoints_previous_date;
mod endpoints_room;

use crate::auth_session::SessionToken;
use crate::data_store::auth_token::Privilege;
use crate::data_store::StoreError;
use actix_web::error::JsonPayloadError;
use actix_web::{
    error::ResponseError,
    http::{header::ContentType, StatusCode},
    web, HttpResponse,
};
use serde_json::json;

pub fn configure_app(cfg: &mut web::ServiceConfig) {
    cfg.service(get_api_service());
}

fn get_api_service() -> actix_web::Scope {
    let json_config =
        web::JsonConfig::default().error_handler(|err, _req| APIError::InvalidJson(err).into());
    web::scope("/api/v1")
        .app_data(json_config)
        .service(endpoints_auth::check_all_events_authorization)
        .service(endpoints_auth::check_authorization)
        .service(endpoints_event::list_events)
        .service(endpoints_event::get_event_info)
        .service(endpoints_event_extended::get_extended_event_info)
        .service(endpoints_event_extended::update_extended_event)
        .service(endpoints_auth::authorize)
        .service(endpoints_auth::drop_access_role)
        .service(endpoints_entry::list_entries)
        .service(endpoints_entry::get_entry)
        .service(endpoints_entry::create_or_update_entry)
        .service(endpoints_entry::change_entry)
        .service(endpoints_entry::delete_entry)
        .service(endpoints_previous_date::create_or_update_previous_date)
        .service(endpoints_previous_date::delete_previous_date)
        .service(endpoints_room::list_rooms)
        .service(endpoints_room::create_or_update_room)
        .service(endpoints_room::delete_room)
        .service(endpoints_category::list_categories)
        .service(endpoints_category::create_or_update_category)
        .service(endpoints_category::delete_category)
        .service(endpoints_announcement::list_announcements)
        .service(endpoints_announcement::create_or_update_announcement)
        .service(endpoints_announcement::change_announcement)
        .service(endpoints_announcement::delete_announcement)
        .service(endpoints_passphrase::list_passphrases)
        .service(endpoints_passphrase::create_passphrase)
        .service(endpoints_passphrase::change_passphrase)
        .service(endpoints_passphrase::delete_passphrase)
}

#[derive(Debug)]
pub enum APIError {
    NotExisting,
    AlreadyExisting,
    PermissionDenied {
        required_privilege: Privilege,
        privilege_expired: bool,
    },
    NoSessionToken,
    InvalidSessionToken,
    AuthenticationFailed {
        passphrase_expired: bool,
    },
    InvalidJson(actix_web::error::JsonPayloadError),
    InvalidData(String),
    EntityIdMissmatch,
    TransactionConflict,
    ConcurrentEditConflict,
    InternalError(String),
}

impl Display for APIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotExisting => f.write_str("Element does not exist")?,
            Self::AlreadyExisting => {
                f.write_str("Element already exists")?;
            },
            Self::PermissionDenied{required_privilege, privilege_expired} => {
                write!(f, "Client is not authorized to perform this action. Authentication as {} is required.{}",
                       required_privilege
                           .qualifying_roles()
                           .iter()
                           .map(|role| role.name().to_owned())
                           .collect::<Vec<String>>()
                           .join(" or "),
                       if *privilege_expired { " The previous authentication for one of these roles has expired." } else { "" })?;
            },
            Self::NoSessionToken => {
                f.write_str("This action requires authentication, but client did not send authentication session token.")?
            },
            Self::InvalidSessionToken => {
                f.write_str("This action requires authentication, but client authentication session given by the client is not valid.")?
            },
            Self::AuthenticationFailed{passphrase_expired} => {
                f.write_str("Authentication with the given passphrase failed.")?;
                if *passphrase_expired {
                    f.write_str(" The passphrase is not yet or no longer valid.")?;
                }
            }
            Self::InternalError(s) => {
                f.write_str("Internal error: ")?;
                f.write_str(s)?;
            },
            Self::InvalidJson(e) => {
                write!(f, "Invalid JSON request data: {}", e)?;
            },
            Self::InvalidData(e) => {
                write!(f, "Invalid request data: {}", e)?;
            },
            Self::EntityIdMissmatch => {
                f.write_str("Entity id in given data does not match URL")?;
            },
            Self::TransactionConflict => {
                f.write_str("Concurrent database transaction conflict. Please retry request.")?;
            },
            Self::ConcurrentEditConflict => {
                f.write_str("Editing entity refused due to a concurrent update of the entity.")?;
            },
        };
        Ok(())
    }
}

impl ResponseError for APIError {
    fn error_response(&self) -> HttpResponse {
        let message = format!("{}", self);

        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(json!({
                "httpCode": self.status_code().as_u16(),
                "message": message
            }))
    }
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotExisting => StatusCode::NOT_FOUND,
            Self::AlreadyExisting => StatusCode::CONFLICT,
            Self::PermissionDenied { .. } => StatusCode::FORBIDDEN,
            Self::NoSessionToken => StatusCode::FORBIDDEN,
            Self::InvalidSessionToken => StatusCode::FORBIDDEN,
            Self::AuthenticationFailed { .. } => StatusCode::FORBIDDEN,
            Self::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InvalidJson(e) => match e {
                JsonPayloadError::ContentType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                JsonPayloadError::Deserialize(json_error) if json_error.is_data() => {
                    StatusCode::UNPROCESSABLE_ENTITY
                }
                _ => StatusCode::BAD_REQUEST,
            },
            &APIError::InvalidData(_) => StatusCode::UNPROCESSABLE_ENTITY,
            &APIError::EntityIdMissmatch => StatusCode::UNPROCESSABLE_ENTITY,
            &APIError::TransactionConflict => StatusCode::SERVICE_UNAVAILABLE,
            Self::ConcurrentEditConflict => StatusCode::CONFLICT,
        }
    }
}

impl From<StoreError> for APIError {
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
            StoreError::NotExisting => Self::NotExisting,
            StoreError::NotValid => Self::NotExisting,
            StoreError::ConflictEntityExists => Self::AlreadyExisting,
            StoreError::ConcurrentEditConflict => Self::ConcurrentEditConflict,
            StoreError::PermissionDenied {
                required_privilege,
                event_id: _,
                privilege_expired,
            } => Self::PermissionDenied {
                required_privilege,
                privilege_expired,
            },
            StoreError::InvalidInputData(e) => Self::InvalidData(e),
            StoreError::InvalidDataInDatabase(e) => Self::InternalError(format!(
                "Data queried from database could not be deserialized: {}",
                e
            )),
        }
    }
}

impl From<actix_web::error::BlockingError> for APIError {
    fn from(_e: actix_web::error::BlockingError) -> Self {
        APIError::InternalError(
            "Could not get thread from thread pool for synchronous database operation.".to_owned(),
        )
    }
}

impl From<crate::auth_session::SessionError> for APIError {
    fn from(_e: crate::auth_session::SessionError) -> Self {
        APIError::InvalidSessionToken
    }
}

struct SessionTokenHeader(String);
#[allow(clippy::identity_op)] // We want to explicitly state that it's "1" year
const SESSION_TOKEN_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(1 * 86400 * 365);

impl SessionTokenHeader {
    fn session_token(
        &self,
        secret: &str,
    ) -> Result<crate::auth_session::SessionToken, crate::auth_session::SessionError> {
        SessionToken::from_string(&self.0, secret, SESSION_TOKEN_MAX_AGE)
    }
}

impl actix_web::http::header::TryIntoHeaderValue for SessionTokenHeader {
    type Error = actix_web::http::header::InvalidHeaderValue;

    fn try_into_value(self) -> Result<actix_web::http::header::HeaderValue, Self::Error> {
        self.0.parse()
    }
}

impl actix_web::http::header::Header for SessionTokenHeader {
    fn name() -> actix_web::http::header::HeaderName {
        "X-SESSION-TOKEN"
            .try_into()
            .expect("Session Token Header name should be a valid header name")
    }

    fn parse<M: actix_web::HttpMessage>(msg: &M) -> Result<Self, actix_web::error::ParseError> {
        Ok(Self(
            msg.headers()
                .get(Self::name())
                .ok_or(actix_web::error::ParseError::Header)?
                .to_str()
                .unwrap_or("")
                .to_owned(),
        ))
    }
}
