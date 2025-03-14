use super::util::{EFFECTIVE_BEGIN_OF_DAY, TIME_BLOCKS, TIME_ZONE};
use super::{AppError, BaseTemplateContext};
use crate::auth_session::SessionToken;
use crate::data_store::models::{FullEntry, Room};
use crate::data_store::{EntryFilter, EntryFilterBuilder};
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use chrono::TimeZone;
use rinja::Template;
use std::collections::BTreeMap;

#[get("/{event_id}/list/{date}")]
async fn main_list(
    path: web::Path<(i32, chrono::NaiveDate)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, date) = path.into_inner();
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
            store.get_entries_filtered(&auth, event_id, date_to_filter(date))?,
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

fn date_to_filter(date: chrono::NaiveDate) -> EntryFilter {
    let begin = date.and_time(EFFECTIVE_BEGIN_OF_DAY);
    let end = begin + chrono::Duration::days(1);
    let mut filter = EntryFilterBuilder::new();
    // TODO handle local time gaps more gracefully – in case we have an event right at DST change
    if let Some(begin) = TIME_ZONE.from_local_datetime(&begin).earliest() {
        filter.after(begin.to_utc());
    }
    if let Some(end) = TIME_ZONE.from_local_datetime(&end).latest() {
        filter.before(end.to_utc());
    }
    filter.build()
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
