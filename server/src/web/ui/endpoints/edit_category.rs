use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, NewCategory};
use crate::data_store::{CategoryId, EventId, StoreError};
use crate::web::ui::base_template::{
    BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::form_values::{BoolFormValue, FormValue, _FormValidSimpleValidate};
use crate::web::ui::sub_templates::form_inputs::{
    CheckboxTemplate, FormFieldTemplate, HiddenInputTemplate, InputConfiguration, InputSize,
    InputType,
};
use crate::web::ui::{util, validation};
use crate::web::AppState;
use actix_web::web::{Form, Html};
use actix_web::{get, post, web, HttpRequest, Responder};
use askama::Template;
use serde::Deserialize;
use uuid::Uuid;

#[get("/{event_id}/config/categories/{category_id}/edit")]
pub async fn edit_category_form(
    path: web::Path<(i32, CategoryId)>,
    state: web::Data<AppState>,
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
            // TODO only get required category
            store.get_event(event_id)?,
            store.get_categories(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let category = categories
        .into_iter()
        .find(|c| c.id == category_id)
        .ok_or(AppError::EntityNotFound)?;
    let form_data: CategoryFormData = category.into();

    let tmpl = EditCategoryFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Kategorie bearbeiten", // TODO
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Categories,
        },
        event_id,
        form_data: &form_data,
        category_id: Some(&category_id),
        has_unsaved_changes: false,
        is_new_category: false,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/config/categories/{category_id}/edit")]
pub async fn edit_category(
    path: web::Path<(EventId, CategoryId)>,
    state: web::Data<AppState>,
    data: Form<CategoryFormData>,
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
            // TODO only get required category
            store.get_event(event_id)?,
            store.get_categories(&auth, event_id)?,
            auth,
        ))
    })
    .await??;
    let _old_category = categories
        .into_iter()
        .find(|c| c.id == category_id)
        .ok_or(AppError::EntityNotFound)?;

    let mut form_data = data.into_inner();
    let category = form_data.validate(Some(category_id));

    let result: util::FormSubmitResult = if let Some(mut category) = category {
        category.event_id = event_id;
        let auth_clone = auth.clone();
        web::block(move || -> Result<_, StoreError> {
            let mut store = state.store.get_facade()?;
            store.create_or_update_category(&auth_clone, category)?;
            Ok(())
        })
        .await?
        .into()
    } else {
        util::FormSubmitResult::ValidationError
    };

    let tmpl = EditCategoryFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Kategorie bearbeiten", // TODO
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Categories,
        },
        event_id,
        form_data: &form_data,
        category_id: Some(&category_id),
        has_unsaved_changes: false,
        is_new_category: false,
    };

    util::create_edit_form_response(
        result,
        &tmpl,
        "Die Kategorie",
        req.url_for(
            "edit_category_form",
            &[event_id.to_string(), category_id.to_string()],
        )?,
        "edit_category_form",
        false,
        req.url_for("manage_categories", &[event_id.to_string()])?,
        &req,
    )
}

#[get("/{event_id}/config/categories/new")]
pub async fn new_category_form(
    path: web::Path<EventId>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageCategories, event_id)?;
    let store = state.store.clone();
    let (event, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageCategories)?;
        Ok((store.get_event(event_id)?, auth))
    })
    .await??;

    let category_id = Uuid::now_v7();
    let form_data: CategoryFormData = CategoryFormData::for_new_category(category_id);

    let tmpl = EditCategoryFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Neue Kategorie", // TODO
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Categories,
        },
        event_id,
        form_data: &form_data,
        category_id: None,
        has_unsaved_changes: false,
        is_new_category: true,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/config/categories/new")]
pub async fn new_category(
    path: web::Path<EventId>,
    state: web::Data<AppState>,
    data: Form<CategoryFormData>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageCategories, event_id)?;
    let store = state.store.clone();
    let (event, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageCategories)?;
        Ok((store.get_event(event_id)?, auth))
    })
    .await??;

    let mut form_data = data.into_inner();
    let category = form_data.validate(None);

    let result: util::FormSubmitResult = if let Some(mut category) = category {
        category.event_id = event_id;
        let auth_clone = auth.clone();
        web::block(move || -> Result<_, StoreError> {
            let mut store = state.store.get_facade()?;
            store.create_or_update_category(&auth_clone, category)?;
            Ok(())
        })
        .await?
        .into()
    } else {
        util::FormSubmitResult::ValidationError
    };

    let tmpl = EditCategoryFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Neue Kategorie", // TODO
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Categories,
        },
        event_id,
        form_data: &form_data,
        category_id: None,
        has_unsaved_changes: true,
        is_new_category: true,
    };

    util::create_edit_form_response(
        result,
        &tmpl,
        "Die Kategorie",
        req.url_for("new_category_form", &[event_id.to_string()])?,
        "edit_category_form",
        true,
        req.url_for("manage_categories", &[event_id.to_string()])?,
        &req,
    )
}

#[derive(Deserialize, Default)]
struct CategoryFormData {
    /// Id of the category, only used for creating new categories (for editing existing entries, the
    /// id is taken from the URL and passed to [validate] as `known_id` instead)
    category_id: FormValue<Uuid>,
    title: FormValue<validation::NonEmptyString>,
    icon: FormValue<String>,
    color: FormValue<validation::ColorHexString>,
    is_official: BoolFormValue,
    sort_key: FormValue<validation::Int32>,
}

impl CategoryFormData {
    fn for_new_category(category_id: CategoryId) -> Self {
        Self {
            category_id: category_id.into(),
            color: validation::ColorHexString("99aabb".to_owned()).into(),
            ..Self::default()
        }
    }

    fn validate(&mut self, known_id: Option<CategoryId>) -> Option<NewCategory> {
        let category_id = known_id.or_else(|| self.category_id.validate());
        let title = self.title.validate();
        let icon = self.icon.validate();
        let color = self.color.validate();
        let is_official = self.is_official.get_value();
        let sort_key = self.sort_key.validate();

        Some(NewCategory {
            id: category_id?,
            title: title?.into_inner(),
            icon: icon?,
            color: color?.0,
            event_id: 0,
            is_official,
            sort_key: sort_key?.0,
        })
    }
}

impl From<Category> for CategoryFormData {
    fn from(value: Category) -> Self {
        Self {
            category_id: value.id.into(),
            title: validation::NonEmptyString(value.title).into(),
            icon: value.icon.into(),
            color: validation::ColorHexString(value.color).into(),
            is_official: value.is_official.into(),
            sort_key: validation::Int32(value.sort_key).into(),
        }
    }
}

#[derive(Template)]
#[template(path = "edit_category_form.html")]
struct EditCategoryFormTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    event_id: EventId,
    form_data: &'a CategoryFormData,
    category_id: Option<&'a CategoryId>,
    has_unsaved_changes: bool,
    is_new_category: bool,
}

impl EditCategoryFormTemplate<'_> {
    fn post_url(&self) -> Result<url::Url, AppError> {
        if self.is_new_category {
            Ok(self
                .base
                .request
                .url_for("new_category", &[self.event_id.to_string()])?)
        } else {
            Ok(self.base.request.url_for(
                "edit_category",
                &[
                    self.event_id.to_string(),
                    self.category_id
                        .expect("For non-new entries, `category_id` should always be known.")
                        .to_string(),
                ],
            )?)
        }
    }
}
