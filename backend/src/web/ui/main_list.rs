use super::{AppError, BaseTemplateContext};
use crate::auth_session::SessionToken;
use crate::data_store::models::{FullEntry, Room};
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use rinja::Template;
use std::collections::BTreeMap;

#[get("/{event_id}/list")]
async fn main_list(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token = SessionToken::from_string(
        req.cookie("kuea-plan-session")
            .ok_or(AppError::NoSession)?
            .value(),
        &state.secret,
        super::SESSION_COOKIE_MAX_AGE,
    )?;
    let (entries, rooms) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.check_authorization(&session_token, event_id)?;
        Ok((
            store.get_entries(&auth, event_id)?,
            store.get_rooms(&auth, event_id)?,
        ))
    })
    .await??;

    let tmpl = MainListTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "TODO",
        },
        entry_blocks: sort_entries_into_blocks(&entries),
        all_entries: entries.iter().collect(),
        rooms: rooms.iter().map(|r| (r.id, r)).collect(),
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "main_list.html")]
struct MainListTemplate<'a> {
    base: BaseTemplateContext<'a>,
    entry_blocks: Vec<(String, Vec<&'a FullEntry>)>,
    all_entries: Vec<&'a FullEntry>,
    rooms: BTreeMap<uuid::Uuid, &'a Room>,
}

fn sort_entries_into_blocks(entries: &Vec<FullEntry>) -> Vec<(String, Vec<&FullEntry>)> {
    // TODO
    vec![("All".to_string(), entries.iter().collect())]
}

mod filters {
    pub fn markdown(input: &str) -> rinja::Result<rinja::filters::Safe<String>> {
        Ok(rinja::filters::Safe(comrak::markdown_to_html(
            input,
            &comrak::ComrakOptions::default(),
        )))
    }
}
