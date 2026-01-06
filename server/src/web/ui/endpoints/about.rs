use crate::web::ui::base_template::{AnyEventData, BaseTemplateContext};
use crate::web::ui::error::AppError;
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use askama::Template;

#[get("/about")]
async fn about_page(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let base_url = req.url_for_static("index")?.to_string();
    let tmpl = AboutTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Über diese Seite",
            event: AnyEventData::None,
            current_date: None,
            auth_token: None,
            active_main_nav_button: None,
        },
        base_url: &base_url,
        admin_name: &state.admin.name,
        admin_email: &state.admin.email,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "about.html")]
struct AboutTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_url: &'a str,
    admin_name: &'a str,
    admin_email: &'a str,
}
