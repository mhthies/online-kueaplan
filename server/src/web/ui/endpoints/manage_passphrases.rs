use crate::data_store::auth_token::Privilege;
use crate::data_store::models::Passphrase;
use crate::data_store::EventId;
use crate::web::ui::base_template::{
    AnyEventData, BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::util;
use crate::web::ui::util::{format_access_role, format_passphrase};
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
    let mut unsorted_passphrases = Vec::new();
    for passphrase in passphrases.iter() {
        if let Some(parent_passphrase_id) = passphrase.derivable_from_passphrase {
            match sorted_passphrases
                .iter_mut()
                .find(|(parent_passphrase, _)| parent_passphrase.id == parent_passphrase_id)
            {
                Some((_, ref mut parent_passphrase_children)) => {
                    parent_passphrase_children.push(passphrase);
                }
                None => {
                    unsorted_passphrases.push(passphrase);
                }
            }
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
        event_id,
        sorted_passphrases: &sorted_passphrases,
        unsorted_passphrases: &unsorted_passphrases,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "manage_passphrases.html")]
struct ManagePassphrasesTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    event_id: EventId,
    sorted_passphrases: &'a PassphrasesWithDerivables<'a>,
    unsorted_passphrases: &'a Vec<&'a Passphrase>,
}
