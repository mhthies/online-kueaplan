use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, FullEntry, Room};
use crate::data_store::{CategoryId, EntryFilter, EventId};
use crate::web::ui::base_template::{BaseTemplateContext, MainNavButton};
use crate::web::ui::error::AppError;
use crate::web::ui::sub_templates::main_list_row::{
    styles_for_category, MainEntryLinkMode, MainListRow, MainListRowTemplate,
};
use crate::web::ui::time_calculation::{get_effective_date, TIME_ZONE};
use crate::web::ui::util;
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use askama::Template;
use std::collections::BTreeMap;

#[get("/{event_id}/categories/{category_id}")]
async fn main_list_by_category(
    path: web::Path<(EventId, CategoryId)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, category_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ShowKueaPlan, event_id)?;
    let (event, entries, rooms, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok((
            store.get_event(event_id)?,
            store.get_entries_filtered(
                &auth,
                event_id,
                EntryFilter::builder()
                    .category_is_one_of(vec![category_id])
                    .build(),
            )?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let category = categories
        .iter()
        .filter(|c| c.id == category_id)
        .next()
        .ok_or(AppError::EntityNotFound)?;
    let title = format!("Kategorie {}", category.title);
    let rows = generate_list_entries(&entries);
    let tmpl = MainListTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: &title,
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::ByCategory),
        },
        entry_blocks: group_rows_into_blocks(&rows),
        entries_with_descriptions: rows
            .iter()
            .filter(|row| {
                row.includes_entry
                    && !row.entry.entry.is_cancelled
                    && !row.entry.entry.description.is_empty()
            })
            .map(|row| row.entry)
            .collect(),
        rooms: rooms.iter().map(|r| (r.id, r)).collect(),
        category,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "main_list_by_category.html")]
struct MainListTemplate<'a> {
    base: BaseTemplateContext<'a>,
    entry_blocks: Vec<(chrono::NaiveDate, Vec<&'a MainListRow<'a>>)>,
    entries_with_descriptions: Vec<&'a FullEntry>,
    rooms: BTreeMap<uuid::Uuid, &'a Room>,
    category: &'a Category,
}

impl<'a> MainListTemplate<'a> {
    fn to_our_timezone(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
        timestamp.with_timezone(&TIME_ZONE).naive_local()
    }
}

/// Filters for the rinja template
mod filters {
    pub use crate::web::ui::askama_filters::markdown;
    use crate::web::ui::util;

    pub fn weekday(date: &chrono::NaiveDate) -> askama::Result<&'static str> {
        Ok(util::weekday(date))
    }
}

/// Generate the list of [MainListRow]s from the given list of KüA-Plan `entries`.
///
/// This algorithm creates a MainListEntry for each entry and each previous_date of an entry,
/// sorts them by `begin` and merges consecutive list rows.
/// This is a simplified version of [super::main_list::generate_list_entries].
fn generate_list_entries<'a>(entries: &'a Vec<FullEntry>) -> Vec<MainListRow<'a>> {
    let mut result = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        result.push(MainListRow::from_entry(entry));
        for previous_date in entry.previous_dates.iter() {
            result.push(MainListRow::from_previous_date(entry, previous_date))
        }
    }
    result.sort_by_key(|e| e.sort_time);
    result.dedup_by(|a, b| {
        if a.entry.entry.id == b.entry.entry.id
            && get_effective_date(a.sort_time) == get_effective_date(b.sort_time)
        {
            b.merge_from(a);
            true
        } else {
            false
        }
    });
    result
}

/// Group the rows of the main list into blocks by effective date.
///
/// The list must be already be sorted by [MainListRow::sort_time].
fn group_rows_into_blocks<'a>(
    entries: &'a Vec<MainListRow<'a>>,
) -> Vec<(chrono::NaiveDate, Vec<&'a MainListRow<'a>>)> {
    let mut result = Vec::new();
    let mut block_entries = Vec::new();
    if entries.is_empty() {
        return result;
    }
    let mut current_date = get_effective_date(&entries[0].sort_time);
    for entry in entries {
        if get_effective_date(&entry.sort_time) != current_date {
            if !block_entries.is_empty() {
                result.push((current_date, block_entries));
            }
            block_entries = Vec::new();
            current_date = get_effective_date(&entry.sort_time);
        }
        block_entries.push(entry);
    }
    result
}
