use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, FullEntry, Room};
use crate::data_store::EntryFilter;
use crate::web::ui::base_template::{BaseTemplateContext, MainNavButton};
use crate::web::ui::error::AppError;
use crate::web::ui::sub_templates::main_list_row::{
    styles_for_category, MainEntryLinkMode, MainListRow, MainListRowTemplate,
};
use crate::web::ui::time_calculation::{
    timestamp_from_effective_date_and_time, EFFECTIVE_BEGIN_OF_DAY, TIME_BLOCKS, TIME_ZONE,
};
use crate::web::ui::util;
use crate::web::ui::util::mark_first_row_of_next_calendar_date;
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use askama::Template;
use chrono::TimeZone;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize, Serialize)]
pub struct MainListQueryData {
    pub after: Option<chrono::NaiveTime>,
}

#[get("/{event_id}/list/{date}")]
async fn main_list(
    path: web::Path<(i32, chrono::NaiveDate)>,
    state: web::Data<AppState>,
    req: HttpRequest,
    query_data: web::Query<MainListQueryData>,
) -> Result<impl Responder, AppError> {
    let (event_id, date) = path.into_inner();
    let time_after = query_data.after;
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ShowKueaPlan, event_id)?;
    let (event, entries, rooms, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok((
            store.get_event(event_id)?,
            store.get_entries_filtered(&auth, event_id, date_to_filter(date, time_after))?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let title = date.format("%d.%m.").to_string();
    let mut rows = generate_filtered_merged_list_entries(&entries, date);
    mark_first_row_of_next_calendar_date(&mut rows, date);
    let tmpl = MainListTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: &title,
            event: Some(&event),
            current_date: Some(date),
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::ByDate),
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
        date,
        time_after,
        footer_constrained_link_times: TIME_BLOCKS
            .iter()
            .filter_map(|b| b.1)
            .filter(|t| *t != EFFECTIVE_BEGIN_OF_DAY)
            .collect(),
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
    date: chrono::NaiveDate,
    time_after: Option<chrono::NaiveTime>,
    footer_constrained_link_times: Vec<chrono::NaiveTime>,
}

impl<'a> MainListTemplate<'a> {
    fn to_our_timezone(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
        timestamp.with_timezone(&TIME_ZONE).naive_local()
    }

    fn link_to_time_constrained_list(
        &self,
        after_time: &chrono::NaiveTime,
    ) -> Result<url::Url, AppError> {
        let mut result = self.base.request.url_for(
            "main_list",
            &[
                self.base
                    .event
                    .expect("Event should always be filled")
                    .id
                    .to_string(),
                self.date.to_string(),
            ],
        )?;
        result.set_query(Some(&serde_urlencoded::to_string(MainListQueryData {
            after: Some(after_time.clone()),
        })?));
        Ok(result)
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

/// Generate an EntryFilter for retrieving only the entries on the given day (using the
/// EFFECTIVE_BEGIN_OF_DAY)
fn date_to_filter(date: chrono::NaiveDate, begin_time: Option<chrono::NaiveTime>) -> EntryFilter {
    let begin =
        timestamp_from_effective_date_and_time(date, begin_time.unwrap_or(EFFECTIVE_BEGIN_OF_DAY));
    let end = date.and_time(EFFECTIVE_BEGIN_OF_DAY) + chrono::Duration::days(1);
    EntryFilter::builder()
        .after(begin)
        .before(
            TIME_ZONE
                .from_local_datetime(&end)
                .latest()
                .map(|dt| dt.to_utc())
                .unwrap_or(end.and_utc()),
        )
        .include_previous_date_matches()
        .build()
}

/// Generate the list of [MainListRow]s for the given `date` from the given list of KüA-Plan
/// `entries`.
///
/// This algorithm creates a MainListEntry for each entry and each previous_date of an entry at the
/// current date, sorts them by `begin` and merges consecutive list rows
fn generate_filtered_merged_list_entries<'a>(
    entries: &'a Vec<FullEntry>,
    date: chrono::NaiveDate,
) -> Vec<MainListRow<'a>> {
    let mut result = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        if effective_date_matches(&entry.entry.begin, &entry.entry.end, date) {
            result.push(MainListRow::from_entry(entry));
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
    use crate::data_store::models::{Entry, FullPreviousDate, PreviousDate};
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
        let result = generate_filtered_merged_list_entries(&entries, "2025-04-28".parse().unwrap());
        assert_eq!(
            result
                .iter()
                .map(|e| {
                    let mut sorted_rooms = e.merged_rooms.clone();
                    sorted_rooms.sort();
                    (
                        e.entry.entry.title.as_str(),
                        e.includes_entry,
                        e.merged_times.len(),
                        sorted_rooms,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("A", false, 1, vec![&room_2]),
                ("C", false, 1, vec![&room_1]),
                ("B", true, 2, vec![&room_3]),
                ("A", true, 1, vec![&room_1, &room_2]),
            ]
        );
    }
}
