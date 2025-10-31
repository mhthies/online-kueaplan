use crate::data_store::auth_token::{AccessRole, Privilege};
use crate::data_store::models::{NewPassphrase, Passphrase};
use crate::data_store::{EventId, StoreError};
use crate::web::ui::base_template::{
    AnyEventData, BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::flash::{FlashMessage, FlashType, FlashesInterface};
use crate::web::ui::form_values::{
    FormValue, FormValueRepresentation, ValidateFromFormInput, _FormValidSimpleValidate,
};
use crate::web::ui::sub_templates::form_inputs::{
    FormFieldTemplate, InputConfiguration, SelectEntry, SelectTemplate,
};
use crate::web::ui::util::{format_access_role, format_passphrase};
use crate::web::ui::{util, validation};
use crate::web::{time_calculation, AppState};
use actix_web::web::{Form, Html, Redirect};
use actix_web::{get, post, web, HttpRequest, Responder};
use askama::Template;
use serde::Deserialize;

#[get("/{event_id}/config/passphrases/new")]
pub async fn new_passphrase_form(
    path: web::Path<EventId>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManagePassphrases, event_id)?;
    let store = state.store.clone();
    let (event, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManagePassphrases)?;
        Ok((store.get_extended_event(&auth, event_id)?, auth))
    })
    .await??;

    let form_data = NewPassphraseFormData::new();

    let tmpl = NewPassphraseFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Neue Passphrase",
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Passphrases,
        },
        form_data: &form_data,
        has_unsaved_changes: false,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/config/passphrases/new")]
pub async fn new_passphrase(
    path: web::Path<EventId>,
    state: web::Data<AppState>,
    data: Form<NewPassphraseFormData>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManagePassphrases, event_id)?;
    let store = state.store.clone();
    let (event, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManagePassphrases)?;
        Ok((store.get_extended_event(&auth, event_id)?, auth))
    })
    .await??;

    let mut form_data = data.into_inner();
    let passphrase = form_data.validate();

    let result: util::FormSubmitResult = if let Some(mut passphrase) = passphrase {
        passphrase.event_id = event_id;
        let auth_clone = auth.clone();
        web::block(move || -> Result<_, StoreError> {
            let mut store = state.store.get_facade()?;
            store.create_passphrase(&auth_clone, passphrase)?;
            Ok(())
        })
        .await?
        .into()
    } else {
        util::FormSubmitResult::ValidationError
    };

    let tmpl = NewPassphraseFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Neue Passphrase",
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Passphrases,
        },
        form_data: &form_data,
        has_unsaved_changes: true,
    };

    util::create_edit_form_response(
        result,
        &tmpl,
        "Die Passphrase",
        req.url_for("new_passphrase_form", &[event_id.to_string()])?,
        "new_passphrase_form",
        true,
        req.url_for("manage_passphrases", &[event_id.to_string()])?,
        &req,
    )
}

#[post("/{event_id}/config/passphrases/{passphrase_id}/new_sharable_link_passphrase")]
pub async fn new_derivable_sharable_link_passphrase(
    path: web::Path<(EventId, i32)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, parent_passphrase_id) = path.into_inner();
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

    let _parent_passphrase = passphrases
        .iter()
        .filter(|p| p.id == parent_passphrase_id)
        .next()
        .ok_or(AppError::EntityNotFound)?;
    let passphrase = NewPassphrase {
        event_id,
        passphrase: None,
        privilege: AccessRole::SharableViewLink,
        derivable_from_passphrase: Some(parent_passphrase_id),
    };

    web::block(move || -> Result<_, StoreError> {
        let mut store = state.store.get_facade()?;
        store.create_passphrase(&auth, passphrase)?;
        Ok(())
    })
    .await??;

    let notification = FlashMessage {
        flash_type: FlashType::Success,
        message: "Die ableitbare Rolle wurde erstellt.".to_string(),
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

#[derive(Debug)]
struct AccessRoleValue(AccessRole);

impl FormValueRepresentation for AccessRoleValue {
    fn into_form_value_string(self) -> String {
        let value: i32 = self.0.into();
        value.to_string()
    }
}
impl ValidateFromFormInput for AccessRoleValue {
    fn from_form_value(value: &str) -> Result<Self, String> {
        let v = value
            .parse::<i32>()
            .map_err(|e| format!("Keine Zahl: {}", e))?;
        Ok(Self(
            v.try_into()
                .map_err(|_| "Keine gültige Rolle".to_string())?,
        ))
    }
}

#[derive(Deserialize)]
struct NewPassphraseFormData {
    access_role: FormValue<AccessRoleValue>,
    passphrase: FormValue<validation::NonEmptyString>,
}

impl NewPassphraseFormData {
    fn new() -> Self {
        Self {
            access_role: AccessRoleValue(AccessRole::User).into(),
            passphrase: Default::default(),
        }
    }

    fn validate(&mut self) -> Option<NewPassphrase> {
        let access_role = self.access_role.validate();
        let passphrase = self.passphrase.validate();

        Some(NewPassphrase {
            event_id: 0,
            passphrase: Some(passphrase?.0),
            privilege: access_role?.0,
            derivable_from_passphrase: None,
        })
    }
}

#[derive(Template)]
#[template(path = "new_passphrase_form.html")]
struct NewPassphraseFormTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    form_data: &'a NewPassphraseFormData,
    has_unsaved_changes: bool,
}

impl NewPassphraseFormTemplate<'_> {
    fn post_url(&self) -> Result<String, AppError> {
        self.base.url_for_event_endpoint("new_passphrase")
    }

    fn role_entries() -> Vec<SelectEntry<'static>> {
        AccessRole::all()
            .filter(|r| r.can_be_managed_online())
            .map(|r| SelectEntry {
                value: i32::from(r.clone()).to_string().into(),
                text: r.name().into(),
            })
            .collect()
    }
}
