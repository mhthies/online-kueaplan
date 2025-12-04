use crate::data_store::auth_token::{AccessRole, Privilege};
use crate::data_store::models::{NewPassphrase, Passphrase};
use crate::data_store::{EventId, StoreError};
use crate::web::ui::base_template::{
    AnyEventData, BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::form_values::{
    FormValue, FormValueRepresentation, ValidateFromFormInput, _FormValidSimpleValidate,
};
use crate::web::ui::sub_templates::form_inputs::{
    FormFieldTemplate, InputType, SelectEntry, SelectTemplate,
};
use crate::web::ui::util::{format_access_role, format_passphrase};
use crate::web::ui::{util, validation};
use crate::web::AppState;
use actix_web::web::{Form, Html};
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
    let passphrase = form_data.validate(&event.clock_info.timezone);

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

#[get("/{event_id}/config/passphrases/{passphrase_id}/new_sharable_link_passphrase")]
pub async fn new_derivable_sharable_link_passphrase_form(
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

    let parent_passphrase = passphrases
        .iter()
        .find(|p| p.id == parent_passphrase_id)
        .ok_or(AppError::EntityNotFound)?;

    let form_data = NewDerivablePassphraseFormData::new();
    let new_access_role = AccessRole::SharableViewLink;
    let title = format!("Neue Ableitbare Rolle für {}", new_access_role.name());

    let tmpl = NewDerivablePassphraseFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: &title,
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
        parent_passphrase,
        new_access_role,
        has_unsaved_changes: false,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/config/passphrases/{passphrase_id}/new_sharable_link_passphrase")]
pub async fn new_derivable_sharable_link_passphrase(
    path: web::Path<(EventId, i32)>,
    state: web::Data<AppState>,
    data: Form<NewDerivablePassphraseFormData>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, parent_passphrase_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManagePassphrases, event_id)?;
    let store = state.store.clone();
    let (passphrases, event, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManagePassphrases)?;
        Ok((
            store.get_passphrases(&auth, event_id)?,
            store.get_extended_event(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let parent_passphrase = passphrases
        .iter()
        .find(|p| p.id == parent_passphrase_id)
        .ok_or(AppError::EntityNotFound)?;
    let mut form_data = data.into_inner();
    let new_access_role = AccessRole::SharableViewLink;
    let passphrase = form_data.validate(&event.clock_info.timezone);

    let result: util::FormSubmitResult = if let Some(mut passphrase) = passphrase {
        passphrase.event_id = event_id;
        passphrase.derivable_from_passphrase = Some(parent_passphrase_id);
        passphrase.privilege = new_access_role;

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

    let title = format!("Neue Ableitbare Rolle für {}", new_access_role.name());
    let tmpl = NewDerivablePassphraseFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: &title,
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
        parent_passphrase,
        new_access_role,
        has_unsaved_changes: false,
    };

    util::create_edit_form_response(
        result,
        &tmpl,
        "Die ableitbare Rolle",
        req.url_for(
            "new_derivable_sharable_link_passphrase_form",
            &[event_id.to_string(), parent_passphrase_id.to_string()],
        )?,
        "new_derivable_passphrase_form",
        true,
        req.url_for("manage_passphrases", &[event_id.to_string()])?,
        &req,
    )
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
    comment: FormValue<String>,
    valid_from: FormValue<validation::MaybeEmpty<validation::DateTimeLocal>>,
    valid_until: FormValue<validation::MaybeEmpty<validation::DateTimeLocal>>,
}

impl NewPassphraseFormData {
    fn new() -> Self {
        Self {
            access_role: AccessRoleValue(AccessRole::User).into(),
            passphrase: Default::default(),
            comment: Default::default(),
            valid_from: Default::default(),
            valid_until: Default::default(),
        }
    }

    fn validate(&mut self, timezone: &chrono_tz::Tz) -> Option<NewPassphrase> {
        let access_role = self.access_role.validate();
        let passphrase = self.passphrase.validate();
        let comment = self.comment.validate();
        let valid_from =
            util::validate_optional_datetime_local_value(&mut self.valid_from, timezone);
        let valid_until =
            util::validate_optional_datetime_local_value(&mut self.valid_until, timezone);

        Some(NewPassphrase {
            event_id: 0,
            passphrase: Some(passphrase?.0),
            privilege: access_role?.0,
            derivable_from_passphrase: None,
            comment: comment?,
            valid_from: valid_from?,
            valid_until: valid_until?,
        })
    }
}

#[derive(Deserialize)]
struct NewDerivablePassphraseFormData {
    comment: FormValue<String>,
    valid_from: FormValue<validation::MaybeEmpty<validation::DateTimeLocal>>,
    valid_until: FormValue<validation::MaybeEmpty<validation::DateTimeLocal>>,
}

impl NewDerivablePassphraseFormData {
    fn new() -> Self {
        Self {
            comment: Default::default(),
            valid_from: Default::default(),
            valid_until: Default::default(),
        }
    }

    fn validate(&mut self, timezone: &chrono_tz::Tz) -> Option<NewPassphrase> {
        let comment = self.comment.validate();
        let valid_from =
            util::validate_optional_datetime_local_value(&mut self.valid_from, timezone);
        let valid_until =
            util::validate_optional_datetime_local_value(&mut self.valid_until, timezone);

        Some(NewPassphrase {
            event_id: 0,
            passphrase: None,
            privilege: AccessRole::SharableViewLink,
            derivable_from_passphrase: None,
            comment: comment?,
            valid_from: valid_from?,
            valid_until: valid_until?,
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
                value: i32::from(*r).to_string().into(),
                text: r.name().into(),
            })
            .collect()
    }
}

#[derive(Template)]
#[template(path = "new_derivable_passphrase_form.html")]
struct NewDerivablePassphraseFormTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    form_data: &'a NewDerivablePassphraseFormData,
    event_id: EventId,
    parent_passphrase: &'a Passphrase,
    new_access_role: AccessRole,
    has_unsaved_changes: bool,
}

impl NewDerivablePassphraseFormTemplate<'_> {
    fn post_url(&self) -> Result<String, AppError> {
        Ok(self
            .base
            .request
            .url_for(
                "new_derivable_sharable_link_passphrase",
                [
                    &self.event_id.to_string(),
                    &self.parent_passphrase.id.to_string(),
                ],
            )?
            .to_string())
    }
}
