use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, FullAnnouncement, FullEntry, Room};
use crate::data_store::{AnnouncementFilter, CategoryId, EntryFilter, EventId};
use crate::web::ui::base_template::{BaseTemplateContext, MainNavButton};
use crate::web::ui::error::AppError;
use crate::web::ui::sub_templates::announcement::AnnouncementTemplate;
use crate::web::ui::sub_templates::main_list_row::{
    styles_for_category, MainEntryLinkMode, MainListRow, MainListRowTemplate,
};
use crate::web::ui::time_calculation::{TIME_ZONE, current_effective_date};
use crate::web::ui::util;
use crate::web::ui::util::mark_first_row_of_next_calendar_date_per_effective_date;
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
    let (event, entries, rooms, categories, announcements, auth) =
        web::block(move || -> Result<_, AppError> {
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
                store.get_announcements(
                    &auth,
                    event_id,
                    Some(AnnouncementFilter::ForCategory(category_id)),
                )?,
                auth,
            ))
        })
        .await??;

    let category = categories
        .iter()
        .find(|c| c.id == category_id)
        .ok_or(AppError::EntityNotFound)?;
    let title = format!("Kategorie {}", category.title);
    let mut rows = util::generate_merged_list_rows_per_date(&entries);
    mark_first_row_of_next_calendar_date_per_effective_date(&mut rows);
    let tmpl = MainListByCategoryTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: &title,
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::ByCategory),
        },
        entry_blocks: util::group_rows_by_date(&rows),
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
        announcements: &announcements,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "main_list_by_category.html")]
struct MainListByCategoryTemplate<'a> {
    base: BaseTemplateContext<'a>,
    entry_blocks: Vec<(chrono::NaiveDate, Vec<&'a MainListRow<'a>>)>,
    entries_with_descriptions: Vec<&'a FullEntry>,
    rooms: BTreeMap<uuid::Uuid, &'a Room>,
    category: &'a Category,
    announcements: &'a Vec<FullAnnouncement>,
}

impl MainListByCategoryTemplate<'_> {
    fn to_our_timezone(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
        timestamp.with_timezone(&TIME_ZONE).naive_local()
    }
}

/// Filters for the rinja template
mod filters {
    pub use crate::web::ui::askama_filters::markdown;
    use crate::web::ui::util;

    pub fn weekday(
        date: &chrono::NaiveDate,
        _: &dyn askama::Values,
    ) -> askama::Result<&'static str> {
        Ok(util::weekday(date))
    }
}
