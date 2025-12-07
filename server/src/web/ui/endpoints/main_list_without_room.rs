use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, ExtendedEvent, FullEntry};
use crate::data_store::{EntryFilter, EventId};
use crate::web::time_calculation::current_effective_date;
use crate::web::ui::base_template::{AnyEventData, BaseTemplateContext, MainNavButton};
use crate::web::ui::error::AppError;
use crate::web::ui::sub_templates::main_list_helpers::EntryDescriptionTemplate;
use crate::web::ui::sub_templates::main_list_row::{
    styles_for_category, MainEntryLinkMode, MainListRow, MainListRowTemplate, RoomByIdWithOrder,
};
use crate::web::ui::util;
use crate::web::ui::util::{
    group_rows_by_date, mark_first_row_of_next_calendar_date_per_effective_date,
};
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use askama::Template;
use std::collections::BTreeMap;

#[get("/{event_id}/rooms/none")]
async fn main_list_without_room(
    path: web::Path<EventId>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ShowKueaPlan, event_id)?;
    let (event, entries, rooms, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok((
            store.get_extended_event(&auth, event_id)?,
            store.get_entries_filtered(
                &auth,
                event_id,
                EntryFilter::builder()
                    .without_room()
                    .include_previous_date_matches()
                    .build(),
            )?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let mut rows = generate_filtered_merged_list_entries(&entries);
    mark_first_row_of_next_calendar_date_per_effective_date(&mut rows, &event.clock_info);
    let tmpl = MainListWithoutRoomTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "ohne Ort",
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::ByRoom),
        },
        entry_blocks: group_rows_by_date(&rows, &event.clock_info),
        entries_with_descriptions: rows
            .iter()
            .filter(|row| {
                row.includes_entry
                    && !row.entry.entry.is_cancelled
                    && !row.entry.entry.description.is_empty()
            })
            .map(|row| row.entry)
            .collect(),
        rooms: rooms.iter().collect(),
        categories: categories.iter().map(|c| (c.id, c)).collect(),
        event: &event,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "main_list_without_room.html")]
struct MainListWithoutRoomTemplate<'a> {
    base: BaseTemplateContext<'a>,
    entry_blocks: Vec<(chrono::NaiveDate, Vec<&'a MainListRow<'a>>)>,
    entries_with_descriptions: Vec<&'a FullEntry>,
    rooms: RoomByIdWithOrder<'a>,
    categories: BTreeMap<uuid::Uuid, &'a Category>,
    event: &'a ExtendedEvent,
}

impl MainListWithoutRoomTemplate<'_> {
    fn to_our_timezone(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
        timestamp
            .with_timezone(&self.event.clock_info.timezone)
            .naive_local()
    }
}

/// Filters for the rinja template
mod filters {
    use crate::web::ui::util;

    pub fn weekday(
        date: &chrono::NaiveDate,
        _: &dyn askama::Values,
    ) -> askama::Result<&'static str> {
        Ok(util::weekday(date))
    }
}

/// Generate the list of [MainListRow]s for the given `room_id` from the given list of KüA-Plan
/// `entries`.
///
/// This algorithm creates a MainListEntry for each entry and each previous_date of an entry without
/// a room, sorts them by `begin` and merges consecutive list rows
fn generate_filtered_merged_list_entries<'a>(entries: &'a [FullEntry]) -> Vec<MainListRow<'a>> {
    let mut result = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        if entry.room_ids.is_empty() {
            result.push(MainListRow::from_entry(entry));
        }
        for previous_date in entry.previous_dates.iter() {
            if previous_date.room_ids.is_empty() {
                result.push(MainListRow::from_previous_date(entry, previous_date))
            }
        }
    }
    result.sort_by_key(|e| e.sort_time);
    result.dedup_by(|a, b| {
        if a.entry.entry.id == b.entry.entry.id {
            b.merge_from(a);
            true
        } else {
            false
        }
    });
    result
}
