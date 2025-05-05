use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, FullEntry, FullPreviousDate, Room};
use crate::data_store::{CategoryId, EntryFilter, RoomId};
use crate::web::ui::base_template::BaseTemplateContext;
use crate::web::ui::colors::CategoryColors;
use crate::web::ui::error::AppError;
use crate::web::ui::time_calculation::{
    timestamp_from_effective_date_and_time, EFFECTIVE_BEGIN_OF_DAY, TIME_BLOCKS, TIME_ZONE,
};
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
    let rows = generate_list_entries(&entries, date);
    let tmpl = MainListTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: &title,
            event: Some(&event),
            current_date: Some(date),
            auth_token: Some(&auth),
        },
        entry_blocks: group_rows_into_blocks(&rows, date),
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
        categories: categories.iter().map(|r| (r.id, r)).collect(),
        timezone: TIME_ZONE,
        date,
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "main_list.html")]
struct MainListTemplate<'a> {
    base: BaseTemplateContext<'a>,
    entry_blocks: Vec<(String, Vec<&'a MainListRow<'a>>)>,
    entries_with_descriptions: Vec<&'a FullEntry>,
    rooms: BTreeMap<uuid::Uuid, &'a Room>,
    categories: BTreeMap<uuid::Uuid, &'a Category>,
    timezone: chrono_tz::Tz,
    date: chrono::NaiveDate,
}

impl<'a> MainListTemplate<'a> {
    fn to_our_timezone(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
        timestamp.with_timezone(&self.timezone).naive_local()
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
    fn css_class_for_entry(&self, row: &'a MainListRow<'a>) -> String {
        let mut result = Self::css_class_for_category(&row.entry.entry.category);
        result.push_str(" kuea-with-category");
        if self
            .categories
            .get(&row.entry.entry.category)
            .map(|c| c.is_official)
            .unwrap_or(false)
        {
            result.push_str(" fw-semibold");
        }
        if !row.includes_entry || row.entry.entry.is_cancelled {
            result.push_str(" kuea-cancelled");
        }
        if row.entry.entry.is_room_reservation {
            result.push_str(" fst-italic");
        }
        result
    }
}

/// Filters for the rinja template
mod filters {
    pub use crate::web::ui::askama_filters::{ellipsis, markdown};
    use crate::web::ui::util;

    pub fn weekday(date: &chrono::NaiveDate) -> askama::Result<&'static str> {
        Ok(util::weekday(date))
    }
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
    filter.include_previous_date_matches();
    filter.build()
}

/// A single row in the list view
///
/// This can either represent a KüA-Plan entry itself at its scheduled time or one or more
/// previous_dates of one (!) entry or a combination of both. The struct does not hold the data
/// itself but only contains references to the [FullEntry] struct and the relevant parts of it.
struct MainListRow<'a> {
    /// The KüA plan entry this row is about
    entry: &'a FullEntry,
    /// The relevant timestamp for sorting this row in the list. I.e. the `begin` of the entry or
    /// the relevant previous_date or the minimum of all of those (when this row covers more than
    /// one begin time)
    sort_time: &'a chrono::DateTime<chrono::Utc>,
    /// `true` if this list row represents the entry itself (with its currently scheduled date),
    /// maybe together with one or more previous dates. `false` if this list entry *only* represents
    /// previous_dates of the KüA-Plan entry
    includes_entry: bool,
    /// The previous_dates represented by this list row (if any)
    previous_dates: Vec<&'a FullPreviousDate>,
    /// The merged set of rooms of all dates represented by this list row
    merged_rooms: Vec<&'a RoomId>,
    /// The set of unique `(begin, end)` times represented by this row that are not equal to the
    /// entry's current scheduled time.
    additional_times: Vec<(
        &'a chrono::DateTime<chrono::Utc>,
        &'a chrono::DateTime<chrono::Utc>,
    )>,
}

impl<'a> MainListRow<'a> {
    /// Create a MainListEntry for given `entry` itself
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

    /// Create a MainListEntry for the given `previous_date` of the `entry`
    fn from_previous_date(entry: &'a FullEntry, previous_date: &'a FullPreviousDate) -> Self {
        debug_assert_eq!(previous_date.previous_date.entry_id, entry.entry.id);
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

    /// Merge two MainListEntries of the same KüA-Plan `entry`.
    ///
    /// This merges all information from `other` into `self`, such that `self` represents all the
    /// dates of the entry (current or previous) of `other` as well, afterward.
    fn merge_from(&mut self, other: &MainListRow<'a>) {
        debug_assert_eq!(self.entry.entry.id, other.entry.entry.id);
        self.sort_time = std::cmp::min(self.sort_time, other.sort_time);
        self.includes_entry |= other.includes_entry;
        self.previous_dates.extend_from_slice(&other.previous_dates);
        for times in other.additional_times.iter() {
            if !self.additional_times.contains(&times)
                && *times != (&self.entry.entry.begin, &self.entry.entry.end)
            {
                self.additional_times.push(*times);
            }
        }
        for room in other.merged_rooms.iter() {
            if !self.merged_rooms.contains(&room) {
                self.merged_rooms.push(room);
            }
        }
    }
}

/// Generate the list of [MainListRow]s for the given `date` from the given list of KüA-Plan
/// `entries`.
///
/// This algorithm creates a MainListEntry for each entry and each previous_date of an entry at the
/// current date, sorts them by `begin` and merges consecutive list rows
fn generate_list_entries<'a>(
    entries: &'a Vec<FullEntry>,
    date: chrono::NaiveDate,
) -> Vec<MainListRow<'a>> {
    let mut result = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        if effective_date_matches(&entry.entry.begin, &entry.entry.end, date) {
            result.push(MainListRow::form_entry(entry));
        }
        for previous_date in entry.previous_dates.iter() {
            if effective_date_matches(
                &previous_date.previous_date.begin,
                &previous_date.previous_date.end,
                date,
            ) {
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

/// Check if the given time interval `(begin, end)` intersects with the given day, using the
/// EFFECTIVE_BEGIN_OF_DAY.
fn effective_date_matches(
    begin: &chrono::DateTime<chrono::Utc>,
    end: &chrono::DateTime<chrono::Utc>,
    effective_date: chrono::NaiveDate,
) -> bool {
    let after = effective_date.and_time(EFFECTIVE_BEGIN_OF_DAY);
    let before = after + chrono::Duration::days(1);
    let after = TIME_ZONE
        .from_local_datetime(&after)
        .earliest()
        .map(|dt| dt.to_utc())
        .unwrap_or(after.and_utc());
    let before = TIME_ZONE
        .from_local_datetime(&before)
        .latest()
        .map(|dt| dt.to_utc())
        .unwrap_or(before.and_utc());

    *end >= after && *begin < before
}

/// Group the rows of the main list into predefined blocks by time
///
/// The list must be already be sorted by [MainListRow::sort_time].
fn group_rows_into_blocks<'a>(
    entries: &'a Vec<MainListRow<'a>>,
    date: chrono::NaiveDate,
) -> Vec<(String, Vec<&'a MainListRow<'a>>)> {
    let mut result = Vec::new();
    let mut block_entries = Vec::new();
    let mut time_block_iter = TIME_BLOCKS.iter();
    let (mut time_block_name, mut time_block_time) = time_block_iter
        .next()
        .expect("At least one time block should be defined.");
    for entry in entries {
        while time_block_time.is_some_and(|block_begin_time| {
            timestamp_from_effective_date_and_time(date, block_begin_time) <= *entry.sort_time
        }) {
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
    use crate::data_store::models::{Entry, PreviousDate};
    use uuid::uuid;
    #[test]
    fn test_generate_list_entries() {
        let room_1 = uuid!("41d96e3c-17de-46ff-9331-690366a4a0a5");
        let room_2 = uuid!("a3820b53-e9a9-4840-b071-7fa3ba34010a");
        let room_3 = uuid!("f6ad3e0b-4371-4a84-a485-45da7f1d8cb8");
        let entries = vec![
            FullEntry {
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
            },
            FullEntry {
                entry: Entry {
                    id: uuid!("01968846-8729-7e19-ae21-6d28e8abde31"),
                    title: "B".to_string(),
                    description: "".to_string(),
                    responsible_person: "".to_string(),
                    is_room_reservation: false,
                    event_id: 1,
                    begin: "2025-04-28 12:00:00+00:00".parse().unwrap(),
                    end: "2025-04-28 13:30:00+00:00".parse().unwrap(),
                    category: Default::default(),
                    last_updated: Default::default(),
                    comment: "".to_string(),
                    time_comment: "".to_string(),
                    room_comment: "".to_string(),
                    is_exclusive: false,
                    is_cancelled: false,
                },
                room_ids: vec![room_3],
                previous_dates: vec![
                    FullPreviousDate {
                        previous_date: PreviousDate {
                            id: uuid!("9eb8121a-9e98-4a54-94da-ed32032a4a91"),
                            entry_id: uuid!("01968846-8729-7e19-ae21-6d28e8abde31"),
                            comment: "Jetzt doch etwas später".to_string(),
                            begin: "2025-04-28 11:30:00+00:00".parse().unwrap(),
                            end: "2025-04-28 13:00:00+00:00".parse().unwrap(),
                        },
                        room_ids: vec![room_3],
                    },
                    FullPreviousDate {
                        previous_date: PreviousDate {
                            id: uuid!("9eb8121a-9e98-4a54-94da-ed32032a4a91"),
                            entry_id: uuid!("01968846-8729-7e19-ae21-6d28e8abde31"),
                            comment: "".to_string(),
                            begin: "2025-04-27 12:00:00+00:00".parse().unwrap(),
                            end: "2025-04-27 13:30:00+00:00".parse().unwrap(),
                        },
                        room_ids: vec![room_3],
                    },
                ],
            },
            FullEntry {
                entry: Entry {
                    id: uuid!("8e17d6dc-1b10-4685-8689-dd998deb17c6"),
                    title: "C".to_string(),
                    description: "".to_string(),
                    responsible_person: "".to_string(),
                    is_room_reservation: false,
                    event_id: 1,
                    begin: "2025-04-27 15:00:00+00:00".parse().unwrap(),
                    end: "2025-04-27 15:30:00+00:00".parse().unwrap(),
                    category: Default::default(),
                    last_updated: Default::default(),
                    comment: "".to_string(),
                    time_comment: "".to_string(),
                    room_comment: "".to_string(),
                    is_exclusive: false,
                    is_cancelled: false,
                },
                room_ids: vec![room_1],
                previous_dates: vec![FullPreviousDate {
                    previous_date: PreviousDate {
                        id: uuid!("9eb8121a-9e98-4a54-94da-ed32032a4a91"),
                        entry_id: uuid!("8e17d6dc-1b10-4685-8689-dd998deb17c6"),
                        comment: "".to_string(),
                        begin: "2025-04-28 11:00:00+00:00".parse().unwrap(),
                        end: "2025-04-28 11:30:00+00:00".parse().unwrap(),
                    },
                    room_ids: vec![room_1],
                }],
            },
        ];
        let result = generate_list_entries(&entries, "2025-04-28".parse().unwrap());
        assert_eq!(
            result
                .iter()
                .map(|e| {
                    let mut sorted_rooms = e.merged_rooms.clone();
                    sorted_rooms.sort();
                    (
                        e.entry.entry.title.as_str(),
                        e.includes_entry,
                        e.additional_times.len(),
                        sorted_rooms,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("A", false, 1, vec![&room_2]),
                ("C", false, 1, vec![&room_1]),
                ("B", true, 1, vec![&room_3]),
                ("A", true, 0, vec![&room_1, &room_2]),
            ]
        );
    }
}
