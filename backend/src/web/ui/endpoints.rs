use super::{AppError, BaseTemplateContext};
use crate::data_store::models::FullEntry;
use actix_web::web::Html;
use actix_web::{get, HttpRequest, Responder};
use rinja::Template;

#[get("/list")]
async fn main_list(req: HttpRequest) -> Result<impl Responder, AppError> {
    let tmpl = MainListTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "TODO",
        },
        entry_blocks: vec![],
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "main_list.html")]
struct MainListTemplate<'a> {
    base: BaseTemplateContext<'a>,
    entry_blocks: Vec<(String, Vec<FullEntry>)>,
}
