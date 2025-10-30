use crate::data_store::auth_token::{AccessRole, Privilege};
use crate::data_store::models::Passphrase;
use crate::web::ui::base_template::{
    AnyEventData, BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::util;
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use askama::Template;

type PassphrasesWithDerivables<'a> = Vec<(&'a Passphrase, Vec<&'a Passphrase>)>;

#[get("/{event_id}/config/passphrases")]
async fn manage_passphrases(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManagePassphrases, event_id)?;
    let (event, passphrases, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManagePassphrases)?;
        Ok((
            store.get_extended_event(&auth, event_id)?,
            store.get_passphrases(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let mut sorted_passphrases: PassphrasesWithDerivables = passphrases
        .iter()
        .filter(|p| p.derivable_from_passphrase.is_none())
        .map(|p| (p, Vec::new()))
        .collect();
    sorted_passphrases.sort_by_key(|p| (p.0.privilege, p.0.id));
    for passphrase in passphrases.iter() {
        if let Some(parent_passphrase_id) = passphrase.derivable_from_passphrase {
            let (_, ref mut parent_passphrase_children) = sorted_passphrases
                .iter_mut()
                .find(|(parent_passphrase, _)| parent_passphrase.id == parent_passphrase_id)
                .ok_or(AppError::InternalError(format!(
                    "Parent passphrase {} pass passphrase {} could not befound.",
                    parent_passphrase_id, passphrase.id
                )))?;
            parent_passphrase_children.push(passphrase);
        }
    }

    let tmpl = ManagePassphrasesTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Passphrasen",
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Passphrases,
        },
        sorted_passphrases: &sorted_passphrases,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "manage_passphrases.html")]
struct ManagePassphrasesTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    sorted_passphrases: &'a PassphrasesWithDerivables<'a>,
}

impl ManagePassphrasesTemplate<'_> {
    fn format_passphrase(passphrase: &Option<String>) -> String {
        passphrase.as_deref().unwrap_or("").replace("\x7f", "*")
    }

    fn format_access_role(role: &AccessRole) -> askama::filters::Safe<String> {
        let (icon, name, color) = match role {
            AccessRole::User => ("person-fill", "Benutzer", "primary"),
            AccessRole::Orga => ("clipboard", "Orga", "warning"),
            AccessRole::Admin => ("gear-fill", "Admin", "warning"),
            AccessRole::SharableViewLink => ("share", "Link-Abruf", "info"),
        };
        askama::filters::Safe(format!(
            "<span class=\"text-{}\"><i class=\"bi bi-{}\"></i> {}</span>",
            color, icon, name
        ))
    }
}
