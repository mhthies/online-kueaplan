use crate::data_store::auth_token::{AccessRole, Privilege};
use crate::data_store::models::Passphrase;
use crate::data_store::{EventId, StoreError};
use crate::web::ui::base_template::{
    AnyEventData, BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::flash::{FlashMessage, FlashType, FlashesInterface};
use crate::web::ui::util;
use crate::web::ui::util::{format_access_role, format_passphrase};
use crate::web::AppState;
use actix_web::web::{Html, Redirect};
use actix_web::{get, post, web, HttpRequest, Responder};
use askama::Template;

#[get("/{event_id}/config/passphrases/{passphrase_id}/delete")]
async fn delete_passphrase_form(
    path: web::Path<(EventId, i32)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, passphrase_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageEntries, event_id)?;
    let store = state.store.clone();
    let (passphrases, event, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageEntries)?;
        Ok((
            store.get_passphrases(&auth, event_id)?,
            store.get_extended_event(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let passphrase = passphrases
        .iter()
        .find(|p| p.id == passphrase_id)
        .ok_or(AppError::EntityNotFound)?;
    if !passphrase.privilege.can_be_managed_online() {
        return Err(AppError::InvalidData(
            "Eine Passphrase mit dieser Rolle kann nicht per Web-Interface gelöscht werden."
                .to_owned(),
        ));
    }
    let parent_passphrase = passphrase
        .derivable_from_passphrase
        .map(|parent_passphrase_id| {
            passphrases
                .iter()
                .find(|p| p.id == parent_passphrase_id)
                .ok_or(AppError::EntityNotFound)
        })
        .transpose()?;

    let tmpl = DeletePassphraseTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: if passphrase.passphrase.is_some() {
                "Passphrase löschen"
            } else {
                "Ableitbare Rolle löschen"
            },
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: None,
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Passphrases,
        },
        event_id,
        passphrase,
        parent_passphrase,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/config/passphrases/{passphrase_id}/delete")]
async fn delete_passphrase(
    path: web::Path<(EventId, i32)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, passphrase_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageEntries, event_id)?;

    web::block(move || -> Result<_, StoreError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        store.delete_passphrase(&auth, event_id, passphrase_id)?;
        Ok(())
    })
    .await??;

    let notification = FlashMessage {
        flash_type: FlashType::Success,
        message: "Die Passphrase/Ableitbare Rolle wurde gelöscht.".to_string(),
        keep_open: false,
        button: None,
    };
    req.add_flash_message(notification);
    Ok(Redirect::to(
        req.url_for("manage_passphrases", &[event_id.to_string()])?
            .to_string(),
    )
    .see_other())
}

#[derive(Template)]
#[template(path = "delete_passphrase_form.html")]
struct DeletePassphraseTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    event_id: EventId,
    passphrase: &'a Passphrase,
    parent_passphrase: Option<&'a Passphrase>,
}
