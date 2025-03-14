use super::{AppError, BaseTemplateContext};
use crate::auth_session::SessionToken;
use crate::data_store::models::{FullEntry, Room};
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use rinja::Template;
use std::collections::BTreeMap;

// TODO move configuration to database / event
const EARLIEST_REASONABLE_KUEA: chrono::NaiveTime =
    chrono::NaiveTime::from_hms_opt(5, 30, 0).unwrap();
const TIME_ZONE: chrono_tz::Tz = chrono_tz::Europe::Berlin;
const TIME_BLOCKS: [(&str, Option<chrono::NaiveTime>); 3] = [
    (
        "Morgens",
        Some(chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap()),
    ),
    (
        "Mittags",
        Some(chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap()),
    ),
    ("Abends", None),
];

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
        timezone: TIME_ZONE,
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
    timezone: chrono_tz::Tz,
}

impl<'a> MainListTemplate<'a> {
    fn to_our_timezone(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
        timestamp.with_timezone(&self.timezone).naive_local()
    }
}

// entries must be sorted by begin timestamp
fn sort_entries_into_blocks(entries: &Vec<FullEntry>) -> Vec<(String, Vec<&FullEntry>)> {
    let mut result = Vec::new();
    let mut block_entries = Vec::new();
    let mut time_block_iter = TIME_BLOCKS.iter();
    let (mut time_block_name, mut time_block_time) = time_block_iter
        .next()
        .expect("At least one time block should be defined.");
    for entry in entries {
        while time_block_time
            .is_some_and(|block_begin_time| entry.entry.begin.time() >= block_begin_time)
        {
            // TODO convert to local timezone
            if !block_entries.is_empty() {
                result.push((time_block_name.to_string(), block_entries));
            }
            (time_block_name, time_block_time) = *time_block_iter
                .next()
                .expect("last time block's time must be 'None' to stop iteration");
            block_entries = Vec::new();
        }
        block_entries.push(entry);
    }
    if !block_entries.is_empty() {
        result.push((time_block_name.to_string(), block_entries));
    }
    result
}

mod filters {
    pub fn markdown(input: &str) -> rinja::Result<rinja::filters::Safe<String>> {
        Ok(rinja::filters::Safe(comrak::markdown_to_html(
            input,
            &comrak::ComrakOptions::default(),
        )))
    }
}
