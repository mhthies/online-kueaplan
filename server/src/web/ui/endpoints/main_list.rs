use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, Event, FullEntry, Room};
use crate::data_store::{CategoryId, EntryFilter};
use crate::web::ui::base_template::BaseTemplateContext;
use crate::web::ui::colors::CategoryColors;
use crate::web::ui::error::AppError;
use crate::web::ui::time_calculation::{EFFECTIVE_BEGIN_OF_DAY, TIME_BLOCKS, TIME_ZONE};
use crate::web::ui::util;
use crate::web::AppState;
use actix_web::error::UrlGenerationError;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use askama::Template;
use chrono::TimeZone;
use std::collections::BTreeMap;

#[get("/{event_id}/list/{date}")]
async fn main_list(
    path: web::Path<(i32, chrono::NaiveDate)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, date) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ShowKueaPlan, event_id)?;
    let (event, entries, rooms, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok((
            store.get_event(&auth, event_id)?,
            store.get_entries_filtered(&auth, event_id, date_to_filter(date))?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let title = date.format("%d.%m.").to_string();
    let tmpl = MainListTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: &title,
        },
        entry_blocks: sort_entries_into_blocks(&entries),
        entries_with_descriptions: entries
            .iter()
            .filter(|e| !e.entry.is_cancelled && !e.entry.description.is_empty())
            .collect(),
        rooms: rooms.iter().map(|r| (r.id, r)).collect(),
        categories: categories.iter().map(|r| (r.id, r)).collect(),
        timezone: TIME_ZONE,
        date,
        event: &event,
        event_days: util::event_days(&event),
        user_can_edit_entries: auth.has_privilege(event_id, Privilege::ManageEntries),
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "main_list.html")]
struct MainListTemplate<'a> {
    base: BaseTemplateContext<'a>,
    entry_blocks: Vec<(String, Vec<&'a FullEntry>)>,
    entries_with_descriptions: Vec<&'a FullEntry>,
    rooms: BTreeMap<uuid::Uuid, &'a Room>,
    categories: BTreeMap<uuid::Uuid, &'a Category>,
    timezone: chrono_tz::Tz,
    date: chrono::NaiveDate,
    event: &'a Event,
    event_days: Vec<chrono::NaiveDate>,
    user_can_edit_entries: bool,
}

impl<'a> MainListTemplate<'a> {
    fn to_our_timezone(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
        timestamp.with_timezone(&self.timezone).naive_local()
    }

    fn url_for_main_list(&self, date: &chrono::NaiveDate) -> Result<String, UrlGenerationError> {
        util::url_for_main_list(self.base.request, self.event.id, date)
    }

    fn url_for_edit_entry(&self, entry: &FullEntry) -> Result<String, UrlGenerationError> {
        util::url_for_edit_entry(self.base.request, entry)
    }

    /// Generate all required (inline) CSS stylesheet content for the given category.
    ///
    /// This function is called within the template once for every category.
    /// It generates CSS rules for the category's CSS class (according to [css_class_for_category])
    /// that can be used for rendering entries belonging to that category.
    fn styles_for_category(category: &Category) -> String {
        let colors = CategoryColors::from_base_color_hex(&category.color)
            .expect("Category color should be a valid HTML hex color string.");
        format!(
            ".{0}{{ {1} }}",
            Self::css_class_for_category(&category.id),
            colors.as_css(),
        )
    }

    /// Return the CSS class name representing the Category with id `category_id`
    fn css_class_for_category(category_id: &CategoryId) -> String {
        format!("category-{}", category_id)
    }

    /// Generate the HTML 'class' attribute for the table row of the given `entry`
    fn css_class_for_entry(entry: &'a FullEntry) -> String {
        let mut result = Self::css_class_for_category(&entry.entry.category);
        result.push_str(" kuea-with-category");
        if entry.entry.is_cancelled {
            result.push_str(" kuea-cancelled");
        }
        if entry.entry.is_room_reservation {
            result.push_str(" fst-italic");
        }
        result
    }
}

/// Filters for the rinja template
mod filters {
    pub use crate::web::ui::askama_filters::{ellipsis, markdown, weekday, weekday_short};
}

/// Generate an EntryFilter for retrieving only the entries on the given day (using the
/// EFFECTIVE_BEGIN_OF_DAY)
fn date_to_filter(date: chrono::NaiveDate) -> EntryFilter {
    let begin = date.and_time(EFFECTIVE_BEGIN_OF_DAY);
    let end = begin + chrono::Duration::days(1);
    let mut filter = EntryFilter::builder();
    filter.after(
        TIME_ZONE
            .from_local_datetime(&begin)
            .earliest()
            .map(|dt| dt.to_utc())
            .unwrap_or(begin.and_utc()),
    );
    filter.before(
        TIME_ZONE
            .from_local_datetime(&end)
            .latest()
            .map(|dt| dt.to_utc())
            .unwrap_or(end.and_utc()),
    );
    filter.build()
}

struct MainListEntry<'a> {
    entry: &'a FullEntry,
    sort_time: &'a chrono::DateTime<chrono::Utc>,
    includes_entry: bool,
    previous_dates: Vec<&'a FullPreviousDate>,
    merged_rooms: Vec<&'a RoomId>,
    additional_times: Vec<(
        &'a chrono::DateTime<chrono::Utc>,
        &'a chrono::DateTime<chrono::Utc>,
    )>,
}

impl<'a> MainListEntry<'a> {
    fn form_entry(entry: &'a FullEntry) -> Self {
        Self {
            entry,
            sort_time: &entry.entry.begin,
            includes_entry: true,
            previous_dates: vec![],
            merged_rooms: entry.room_ids.iter().collect(),
            additional_times: vec![],
        }
    }

    fn from_previous_date(entry: &'a FullEntry, previous_date: &'a FullPreviousDate) -> Self {
        Self {
            entry,
            sort_time: &previous_date.previous_date.begin,
            includes_entry: false,
            previous_dates: vec![previous_date],
            merged_rooms: previous_date.room_ids.iter().collect(),
            additional_times: vec![(
                &previous_date.previous_date.begin,
                &previous_date.previous_date.end,
            )],
        }
    }

    fn merge_from(&mut self, other: &mut MainListEntry<'a>) {
        assert_eq!(self.entry.entry.id, other.entry.entry.id);
        self.sort_time = std::cmp::min(self.sort_time, other.sort_time);
        self.includes_entry |= other.includes_entry;
        self.previous_dates.append(&mut other.previous_dates);
        // TODO deduplicate additional_times (with existing and with entry's begin/end)
        self.additional_times.append(&mut other.additional_times);
        for room in std::mem::take(&mut other.merged_rooms) {
            if !self.merged_rooms.contains(&room) {
                self.merged_rooms.push(room);
            }
        }
    }
}

fn generate_list_entries(entries: &Vec<FullEntry>) -> Vec<MainListEntry> {
    let mut result = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        // TODO filter by date
        result.push(MainListEntry::form_entry(entry));
        for previous_date in entry.previous_dates.iter() {
            // TODO filter by date
            result.push(MainListEntry::from_previous_date(entry, previous_date))
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

// entries must be sorted by begin timestamp
fn sort_entries_into_blocks(entries: &Vec<FullEntry>) -> Vec<(String, Vec<&FullEntry>)> {
    let mut result = Vec::new();
    let mut block_entries = Vec::new();
    let mut time_block_iter = TIME_BLOCKS.iter();
    let (mut time_block_name, mut time_block_time) = time_block_iter
        .next()
        .expect("At least one time block should be defined.");
    for entry in entries {
        while time_block_time.is_some_and(|block_begin_time| {
            entry.entry.begin.with_timezone(&TIME_ZONE).time() >= block_begin_time
        }) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_store::models::Entry;
    use uuid::uuid;
    #[test]
    fn test_generate_list_entries() {
        let room_1 = uuid!("f6ad3e0b-4371-4a84-a485-45da7f1d8cb8");
        let room_2 = uuid!("a3820b53-e9a9-4840-b071-7fa3ba34010a");
        let room_3 = uuid!("41d96e3c-17de-46ff-9331-690366a4a0a5");
        let entries = vec![FullEntry {
            entry: Entry {
                id: uuid!("05c93b6e-29ad-4ace-8a32-244723973331"),
                title: "A".to_string(),
                description: "".to_string(),
                responsible_person: "".to_string(),
                is_room_reservation: false,
                event_id: 1,
                begin: "2025-04-28 14:00:00+00:00".parse().unwrap(),
                end: "2025-04-28 16:00:00+00:00".parse().unwrap(),
                category: Default::default(),
                last_updated: Default::default(),
                comment: "".to_string(),
                time_comment: "".to_string(),
                room_comment: "".to_string(),
                is_exclusive: false,
                is_cancelled: false,
            },
            room_ids: vec![room_1],
            previous_dates: vec![
                FullPreviousDate {
                    previous_date: PreviousDate {
                        id: uuid!("6385b911-c641-47c8-8d50-26d2fe1ee764"),
                        entry_id: uuid!("05c93b6e-29ad-4ace-8a32-244723973331"),
                        comment: "Wegen Kollision mit Anreise auf Nachmittag verschoben"
                            .to_string(),
                        begin: "2025-04-28 9:00:00+00:00".parse().unwrap(),
                        end: "2025-04-28 10:00:00+00:00".parse().unwrap(),
                    },
                    room_ids: vec![room_2],
                },
                FullPreviousDate {
                    previous_date: PreviousDate {
                        id: uuid!("38023800-c9be-45a8-8d08-2f118ea6b15c"),
                        entry_id: uuid!("05c93b6e-29ad-4ace-8a32-244723973331"),
                        comment: "Klavier steht jetzt in Raum 1".to_string(),
                        begin: "2025-04-28 14:00:00+00:00".parse().unwrap(),
                        end: "2025-04-28 16:00:00+00:00".parse().unwrap(),
                    },
                    room_ids: vec![room_2],
                },
            ],
        }];
        // TODO two more entries: one with two previous dates (one on other date, one near event),
        //   one on other day with previous date on current date.
        let result = generate_list_entries(&entries);
        // TODO check result
    }
}
