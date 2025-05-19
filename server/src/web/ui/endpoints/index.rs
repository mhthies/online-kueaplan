use crate::data_store::auth_token::Privilege;
use crate::data_store::EventId;
use crate::web::ui::error::AppError;
use crate::web::ui::{time_calculation, util};
use crate::web::AppState;
use actix_web::web::Redirect;
use actix_web::{get, web, HttpRequest, Responder};

#[get("/{event_id}")]
async fn index(
    path: web::Path<EventId>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token_if_present(&state, &req, Privilege::ShowKueaPlan, event_id)?;
    let (event, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = session_token
            .map(|token| store.get_auth_token_for_session(&token, event_id))
            .transpose()?;
        Ok((store.get_event(event_id)?, auth))
    })
    .await??;

    if auth.is_some_and(|auth| auth.has_privilege(event_id, Privilege::ShowKueaPlan)) {
        Ok(Redirect::to(
            req.url_for(
                "main_list",
                &[
                    event_id.to_string(),
                    time_calculation::most_reasonable_date(&event).to_string(),
                ],
            )?
            .to_string(),
        )
        .see_other())
    } else {
        Ok(Redirect::to(
            req.url_for("login_form", &[event_id.to_string()])?
                .to_string(),
        )
        .see_other())
    }
}
