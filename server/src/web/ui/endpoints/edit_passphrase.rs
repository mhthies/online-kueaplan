use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Passphrase, PassphrasePatch};
use crate::data_store::{EventId, PassphraseId, StoreError};
use crate::web::ui::base_template::{
    AnyEventData, BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::form_values::{FormValue, _FormValidSimpleValidate};
use crate::web::ui::sub_templates::form_inputs::{FormFieldTemplate, InputType};
use crate::web::ui::util::{format_access_role, format_passphrase};
use crate::web::ui::{util, validation};
use crate::web::AppState;
use actix_web::web::{Form, Html};
use actix_web::{get, post, web, HttpRequest, Responder};
use askama::Template;
use serde::Deserialize;

#[get("/{event_id}/config/passphrases/{passphrase_id}/edit")]
pub async fn edit_passphrase_form(
    path: web::Path<(EventId, PassphraseId)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, passphrase_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManagePassphrases, event_id)?;
    let store = state.store.clone();
    let (event, passphrases, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManagePassphrases)?;
        Ok((
            store.get_extended_event(&auth, event_id)?,
            store.get_passphrases(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let (passphrase, parent_passphrase) =
        find_passphrase_and_check_privilege(&passphrases, passphrase_id)?;
    let form_data =
        EditPassphraseFormData::for_existing_passphrase(&passphrase, &event.clock_info.timezone);

    let tmpl = EditPassphraseFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: if passphrase.passphrase.is_some() {
                "Passphrase bearbeiten"
            } else {
                "Ableitbare Rolle bearbeiten"
            },
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Passphrases,
        },
        form_data: &form_data,
        event_id,
        passphrase,
        parent_passphrase,
        has_unsaved_changes: false,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/config/passphrases/{passphrase_id}/edit")]
pub async fn edit_passphrase(
    path: web::Path<(EventId, PassphraseId)>,
    state: web::Data<AppState>,
    data: Form<EditPassphraseFormData>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, passphrase_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManagePassphrases, event_id)?;
    let store = state.store.clone();
    let (event, passphrases, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManagePassphrases)?;
        Ok((
            store.get_extended_event(&auth, event_id)?,
            store.get_passphrases(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let (passphrase, parent_passphrase) =
        find_passphrase_and_check_privilege(&passphrases, passphrase_id)?;
    let mut form_data = data.into_inner();
    let passphrase_patch = form_data.validate(&event.clock_info.timezone);

    let result: util::FormSubmitResult = if let Some(passphrase_patch) = passphrase_patch {
        let auth_clone = auth.clone();
        web::block(move || -> Result<_, StoreError> {
            let mut store = state.store.get_facade()?;
            store.patch_passphrase(&auth_clone, passphrase_id, passphrase_patch)?;
            Ok(())
        })
        .await?
        .into()
    } else {
        util::FormSubmitResult::ValidationError
    };

    let tmpl = EditPassphraseFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: if passphrase.passphrase.is_some() {
                "Passphrase bearbeiten"
            } else {
                "Ableitbare Rolle bearbeiten"
            },
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Passphrases,
        },
        form_data: &form_data,
        event_id,
        passphrase,
        parent_passphrase,
        has_unsaved_changes: true,
    };

    util::create_edit_form_response(
        result,
        &tmpl,
        "Die Passphrase",
        req.url_for(
            "edit_passphrase_form",
            &[event_id.to_string(), passphrase_id.to_string()],
        )?,
        "edit_passphrase_form",
        false,
        req.url_for("manage_passphrases", &[event_id.to_string()])?,
        &req,
    )
}

/// Helper function for both edit_passphrase endpoints:
/// Returns the passphrase to be edited, it's parent_passphrase (if any) and checks that the
/// passphrase is allowed to be edited online.
fn find_passphrase_and_check_privilege(
    passphrases: &[Passphrase],
    passphrase_id: PassphraseId,
) -> Result<(&Passphrase, Option<&Passphrase>), AppError> {
    let passphrase = passphrases
        .iter()
        .find(|p| p.id == passphrase_id)
        .ok_or(AppError::EntityNotFound)?;
    if !passphrase.privilege.can_be_managed_online() {
        return Err(AppError::InvalidData(
            "Eine Passphrase mit dieser Rolle kann nicht per Web-Interface bearbeitet werden."
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
    Ok((passphrase, parent_passphrase))
}

#[derive(Deserialize)]
struct EditPassphraseFormData {
    comment: FormValue<String>,
    valid_from: FormValue<validation::MaybeEmpty<validation::DateTimeLocal>>,
    valid_until: FormValue<validation::MaybeEmpty<validation::DateTimeLocal>>,
}

impl EditPassphraseFormData {
    fn for_existing_passphrase(passphrase: &Passphrase, timezone: &chrono_tz::Tz) -> Self {
        Self {
            comment: passphrase.comment.clone().into(),
            valid_from: validation::MaybeEmpty(
                passphrase
                    .valid_from
                    .clone()
                    .map(|t| validation::DateTimeLocal(t.with_timezone(timezone).naive_local())),
            )
            .into(),
            valid_until: validation::MaybeEmpty(
                passphrase
                    .valid_until
                    .clone()
                    .map(|t| validation::DateTimeLocal(t.with_timezone(timezone).naive_local())),
            )
            .into(),
        }
    }

    fn validate(&mut self, timezone: &chrono_tz::Tz) -> Option<PassphrasePatch> {
        let comment = self.comment.validate();
        let valid_from =
            util::validate_optional_datetime_local_value(&mut self.valid_from, timezone);
        let valid_until =
            util::validate_optional_datetime_local_value(&mut self.valid_until, timezone);

        Some(PassphrasePatch {
            comment: Some(comment?),
            valid_from: Some(valid_from?),
            valid_until: Some(valid_until?),
        })
    }
}

#[derive(Template)]
#[template(path = "edit_passphrase_form.html")]
struct EditPassphraseFormTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    form_data: &'a EditPassphraseFormData,
    event_id: EventId,
    passphrase: &'a Passphrase,
    parent_passphrase: Option<&'a Passphrase>,
    has_unsaved_changes: bool,
}

impl EditPassphraseFormTemplate<'_> {
    fn post_url(&self) -> Result<String, AppError> {
        Ok(self
            .base
            .request
            .url_for(
                "edit_passphrase",
                &[&self.event_id.to_string(), &self.passphrase.id.to_string()],
            )?
            .to_string())
    }
}
