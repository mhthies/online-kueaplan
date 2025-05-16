use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, FullEntry};
use crate::data_store::{CategoryId, EntryFilter, EventId};
use crate::web::ui::base_template::{
    BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::form_values::FormValue;
use crate::web::ui::sub_templates::form_inputs::{InputConfiguration, SelectEntry, SelectTemplate};
use crate::web::ui::time_calculation::TIME_ZONE;
use crate::web::ui::{util, validation};
use crate::web::AppState;
use actix_web::web::{Form, Html};
use actix_web::{get, post, web, HttpRequest, Responder};
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
    let (event, categories, mut category_entries, auth) =
        web::block(move || -> Result<_, AppError> {
            let mut store = state.store.get_facade()?;
            let auth = store.get_auth_token_for_session(&session_token, event_id)?;
            auth.check_privilege(event_id, Privilege::ManageCategories)?;
            Ok((
                store.get_event(&auth, event_id)?,
                store.get_categories(&auth, event_id)?,
                store.get_entries_filtered(&auth, event_id, entry_filter)?,
                auth,
            ))
        })
        .await??;

    let category = categories
        .iter()
        .filter(|c| c.id == category_id)
        .next()
        .ok_or(AppError::EntityNotFound)?;
    // TODO allow sorting by database
    category_entries.sort_by_key(|e| e.entry.begin);

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
    todo!();
    Ok(Html::new(""))
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

impl<'a> DeleteCategoryFormTemplate<'a> {
    fn other_category_entries(&self) -> Vec<SelectEntry> {
        self.all_categories
            .iter()
            .filter(|c| c.id != self.category.id)
            .map(|c| SelectEntry {
                value: Cow::Owned(c.id.to_string().into()),
                text: Cow::Borrowed(c.title.as_str()),
            })
            .collect()
    }

    fn post_url(&self) -> Result<url::Url, AppError> {
        Ok(self.base.request.url_for(
            "delete_category",
            &[&self.event_id.to_string(), &self.category.id.to_string()],
        )?)
    }

    fn to_our_timezone(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
        timestamp.with_timezone(&TIME_ZONE).naive_local()
    }
}
