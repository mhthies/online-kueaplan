use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, FullEntry};
use crate::data_store::{CategoryId, EntryFilter, EventId};
use crate::web::ui::base_template::{
    BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::flash::{FlashMessage, FlashType, FlashesInterface};
use crate::web::ui::form_values::FormValue;
use crate::web::ui::sub_templates::form_inputs::{InputConfiguration, SelectEntry, SelectTemplate};
use crate::web::time_calculation::TIME_ZONE;
use crate::web::ui::{util, validation};
use crate::web::AppState;
use actix_web::web::{Form, Html, Redirect};
use actix_web::{get, post, web, Either, HttpRequest, Responder};
use askama::Template;
use serde::Deserialize;
use std::borrow::Cow;

#[get("/{event_id}/config/categories/{category_id}/delete")]
pub async fn delete_category_form(
    path: web::Path<(i32, CategoryId)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, category_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageCategories, event_id)?;
    let entry_filter = EntryFilter::builder()
        .category_is_one_of(vec![category_id])
        .build();
    let (event, categories, category_entries, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageCategories)?;
        Ok((
            store.get_event(event_id)?,
            store.get_categories(&auth, event_id)?,
            store.get_entries_filtered(&auth, event_id, entry_filter)?,
            auth,
        ))
    })
    .await??;

    let category = categories
        .iter()
        .find(|c| c.id == category_id)
        .ok_or(AppError::EntityNotFound)?;
    if categories.len() == 1 {
        return Err(AppError::InvalidData(
            "Die letzte Kategorie darf nicht gelöscht werden.".to_owned(),
        ));
    }

    let form_data = DeleteCategoryFormData::default();

    let tmpl = DeleteCategoryFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Kategorie löschen", // TODO
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Categories,
        },
        event_id,
        category,
        all_categories: &categories,
        category_entries: &category_entries,
        form_data: &form_data,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/config/categories/{category_id}/delete")]
pub async fn delete_category(
    path: web::Path<(EventId, CategoryId)>,
    state: web::Data<AppState>,
    data: Form<DeleteCategoryFormData>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, category_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageCategories, event_id)?;
    let store = state.store.clone();
    let (event, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageCategories)?;
        Ok((
            store.get_event(event_id)?,
            store.get_categories(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let mut form_data = data.into_inner();
    let replacement_category = form_data.replace_category.validate_with(
        &categories
            .iter()
            .filter(|c| c.id != category_id)
            .map(|c| c.id)
            .collect::<Vec<CategoryId>>(),
    );

    let result = if let Some(replacement_category) = replacement_category {
        let store = state.store.clone();
        let auth = auth.clone();
        Some(
            web::block(move || -> Result<_, AppError> {
                let mut store = store.get_facade()?;
                store.delete_category(
                    &auth,
                    event_id,
                    category_id,
                    Some(replacement_category.into_inner()),
                )?;
                Ok(())
            })
            .await?,
        )
    } else {
        None
    };

    match result {
        Some(Ok(())) => {
            let notification = FlashMessage {
                flash_type: FlashType::Success,
                message: "Die Kategorie wurde gelöscht.".to_string(),
                keep_open: false,
                button: None,
            };
            req.add_flash_message(notification);
            return Ok(Either::Left(
                Redirect::to(
                    req.url_for("manage_categories", [&event_id.to_string()])?
                        .to_string(),
                )
                .see_other(),
            ));
        }
        None => {
            let notification = FlashMessage {
                flash_type: FlashType::Error,
                message: "Eingegebene Daten sind ungültig. Bitte markierte Felder überprüfen."
                    .to_owned(),
                keep_open: false,
                button: None,
            };
            req.add_flash_message(notification);
        }
        Some(Err(e)) => match e {
            AppError::TransactionConflict => {
                let notification = FlashMessage {
                    flash_type: FlashType::Error,
                    message: "Die Kategorie konnte wegen eines parallelen Datenbank-Zugriff nicht gelöscht werden. Bitte erneut versuchen.".to_string(),
                    keep_open: true,
                    button: None,
                };
                req.add_flash_message(notification);
            }
            _ => {
                return Err(e);
            }
        },
    };

    // TODO deduplicate code with delete_category_form
    let entry_filter = EntryFilter::builder()
        .category_is_one_of(vec![category_id])
        .build();
    let store = state.store.clone();
    let (mut category_entries, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        auth.check_privilege(event_id, Privilege::ManageCategories)?;
        Ok((
            store.get_entries_filtered(&auth, event_id, entry_filter)?,
            auth,
        ))
    })
    .await??;

    let category = categories
        .iter()
        .find(|c| c.id == category_id)
        .ok_or(AppError::EntityNotFound)?;
    category_entries.sort_by_key(|e| e.entry.begin);

    let tmpl = DeleteCategoryFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Kategorie löschen", // TODO
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Categories,
        },
        event_id,
        category,
        all_categories: &categories,
        category_entries: &category_entries,
        form_data: &form_data,
    };

    Ok(Either::Right(Html::new(tmpl.render()?)))
}

#[derive(Deserialize, Default)]
struct DeleteCategoryFormData {
    replace_category: FormValue<validation::UuidFromList>,
}

#[derive(Template)]
#[template(path = "delete_category_form.html")]
struct DeleteCategoryFormTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    event_id: EventId,
    category: &'a Category,
    all_categories: &'a Vec<Category>,
    category_entries: &'a Vec<FullEntry>,
    form_data: &'a DeleteCategoryFormData,
}

impl DeleteCategoryFormTemplate<'_> {
    fn other_category_entries(&self) -> Vec<SelectEntry> {
        self.all_categories
            .iter()
            .filter(|c| c.id != self.category.id)
            .map(|c| SelectEntry {
                value: Cow::Owned(c.id.to_string()),
                text: Cow::Borrowed(c.title.as_str()),
            })
            .collect()
    }

    fn post_url(&self) -> Result<url::Url, AppError> {
        Ok(self.base.request.url_for(
            "delete_category",
            [&self.event_id.to_string(), &self.category.id.to_string()],
        )?)
    }

    fn to_our_timezone(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
        timestamp.with_timezone(&TIME_ZONE).naive_local()
    }
}
