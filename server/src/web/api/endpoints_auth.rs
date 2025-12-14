use crate::auth_session::SessionToken;
use crate::data_store::StoreError;
use crate::web::api::{APIError, SessionTokenHeader};
use crate::web::AppState;
use actix_web::{get, post, web, Responder};
use kueaplan_api_types::{
    AllEventsAuthorizationInfo, Authorization, AuthorizationInfo, AuthorizationRole,
};
use serde::{Deserialize, Serialize};

#[get("/auth")]
async fn check_all_events_authorization(
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let session_token = session_token_header
        .map(|token_header| token_header.into_inner().session_token(&state.secret))
        .transpose()?;
    let mut raw_authorization_list = if let Some(token) = session_token {
        web::block(move || -> Result<_, APIError> {
            let mut store = state.store.get_facade()?;
            Ok(store.list_all_access_roles(&token)?)
        })
        .await??
    } else {
        vec![]
    };
    raw_authorization_list.sort();
    let events: Vec<AuthorizationInfo> =
        raw_authorization_list
            .into_iter()
            .fold(vec![], |mut accum, (event_id, role)| {
                match accum.last_mut() {
                    Some(current_entry) if current_entry.event_id == event_id => {
                        current_entry
                            .authorization
                            .push(Authorization { role: role.into() });
                    }
                    _ => {
                        accum.push(AuthorizationInfo {
                            event_id,
                            authorization: vec![Authorization { role: role.into() }],
                        });
                    }
                };
                accum
            });

    Ok(web::Json(AllEventsAuthorizationInfo { events }))
}

#[get("/events/{eventId}/auth")]
async fn check_authorization(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let event_id = path.into_inner();
    let session_token = session_token_header
        .map(|token_header| token_header.into_inner().session_token(&state.secret))
        .transpose()?;
    let authorization: Vec<kueaplan_api_types::Authorization> = if let Some(token) = session_token {
        web::block(move || -> Result<_, APIError> {
            let mut store = state.store.get_facade()?;
            let auth = store.get_auth_token_for_session(&token, event_id)?;
            Ok(auth.list_api_access_roles())
        })
        .await??
    } else {
        vec![]
    };
    let authorization_info = AuthorizationInfo {
        event_id,
        authorization,
    };
    Ok(web::Json(authorization_info))
}

#[derive(Deserialize)]
struct AuthorizeRequest {
    passphrase: String,
}
#[derive(Serialize)]
struct AuthorizeResponse {
    #[serde(flatten)]
    authorization_info: AuthorizationInfo,
    #[serde(rename = "sessionToken")]
    session_token: String,
}

#[post("/events/{eventId}/auth")]
async fn authorize(
    path: web::Path<i32>,
    body: web::Json<AuthorizeRequest>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let event_id = path.into_inner();
    let session_token = session_token_header
        .map(|token_header| token_header.into_inner().session_token(&state.secret))
        .transpose()?
        .unwrap_or_else(SessionToken::new);
    let store = state.store.clone();
    let (authorization, session_token) = {
        web::block(move || -> Result<_, APIError> {
            let mut session_token = session_token;
            let mut store = store.get_facade()?;
            store
                .authenticate_with_passphrase(event_id, &body.passphrase, &mut session_token)
                .map_err(|e| match e {
                    StoreError::NotExisting => APIError::AuthenticationFailed {
                        passphrase_expired: false,
                    },
                    StoreError::NotValid => APIError::AuthenticationFailed {
                        passphrase_expired: true,
                    },
                    e => e.into(),
                })?;
            let auth = store.get_auth_token_for_session(&session_token, event_id)?;
            Ok((auth.list_api_access_roles(), session_token))
        })
        .await??
    };
    Ok(web::Json(AuthorizeResponse {
        authorization_info: AuthorizationInfo {
            event_id,
            authorization,
        },
        session_token: session_token.as_string(&state.secret),
    }))
}

#[derive(Deserialize)]
struct DropAccessRoleRequest {
    role: AuthorizationRole,
}

#[post("/events/{eventId}/dropAccessRole")]
async fn drop_access_role(
    path: web::Path<i32>,
    body: web::Json<DropAccessRoleRequest>,
    state: web::Data<AppState>,
    session_token_header: Option<web::Header<SessionTokenHeader>>,
) -> Result<impl Responder, APIError> {
    let event_id = path.into_inner();
    let session_token = session_token_header
        .map(|token_header| token_header.into_inner().session_token(&state.secret))
        .transpose()?
        .unwrap_or_else(SessionToken::new);
    let store = state.store.clone();
    let (authorization, session_token) = {
        web::block(move || -> Result<_, APIError> {
            let mut session_token = session_token;
            let mut store = store.get_facade()?;
            store.drop_access_role(event_id, body.role.into(), &mut session_token)?;
            let auth = store.get_auth_token_for_session(&session_token, event_id)?;
            Ok((auth.list_api_access_roles(), session_token))
        })
        .await??
    };
    Ok(web::Json(AuthorizeResponse {
        authorization_info: AuthorizationInfo {
            event_id,
            authorization,
        },
        session_token: session_token.as_string(&state.secret),
    }))
}
