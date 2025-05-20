use crate::data_store::auth_token::Privilege;
use crate::data_store::models::Category;
use crate::data_store::EventId;
use crate::web::ui::base_template::{BaseTemplateContext, MainNavButton};
use crate::web::ui::colors::CategoryColors;
use crate::web::ui::error::AppError;
use crate::web::ui::util;
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use askama::Template;

#[get("/{event_id}/categories")]
async fn categories_list(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ShowKueaPlan, event_id)?;
    let (event, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ShowKueaPlan)?;
        Ok((
            store.get_event(event_id)?,
            store.get_categories(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let tmpl = CategoriesListTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Kategorien",
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::ByCategory),
        },
        event_id,
        categories: &categories,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "categories_list.html")]
struct CategoriesListTemplate<'a> {
    base: BaseTemplateContext<'a>,
    event_id: EventId,
    categories: &'a Vec<Category>,
}
