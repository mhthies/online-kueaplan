use crate::auth_session::SessionToken;
use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, EventClockInfo, ExtendedEvent, FullEntry, Room};
use crate::data_store::{CategoryId, EntryFilter, EntryId, EventId, RoomId};
use crate::web::time_calculation::get_effective_date;
use crate::web::ui::error::AppError;
use crate::web::AppState;
use actix_web::error::UrlGenerationError;
use actix_web::http::header::DispositionParam;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpRequest, HttpResponseBuilder, Responder};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[allow(clippy::identity_op)] // We want to explicitly state that it's "1" year
pub const SESSION_COOKIE_MAX_AGE: std::time::Duration =
    std::time::Duration::from_secs(1 * 86400 * 365);

#[get("/events/{event_id}/frab-xml")]
async fn frab_xml(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    query: web::Query<FrabXmlQueryParams>,
    http_request: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let query = query.into_inner();
    let session_token =
        SessionToken::from_string(&query.session_token, &state.secret, SESSION_COOKIE_MAX_AGE)
            .map_err(|session_error| AppError::PermissionDenied {
                required_privilege: Privilege::ShowKueaPlan,
                event_id,
                session_error: Some(session_error),
            })?;

    let url_for_event = |event_id: &EventId| {
        http_request
            .url_for("event_index", [&event_id.to_string()])
            .unwrap()
            .to_string()
    };
    let url_for_entry = |entry: &FullEntry, event: &ExtendedEvent| {
        xml_entry_url(
            &http_request,
            event_id,
            &entry.entry.id,
            &get_effective_date(&entry.entry.begin, &event.clock_info),
        )
        .unwrap()
        .to_string()
    };

    let (event, entries, rooms, categories) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok((
            store.get_extended_event(&auth, event_id)?,
            store.get_entries_filtered(&auth, event_id, EntryFilter::default())?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
        ))
    })
    .await??;
    let categories_by_id: BTreeMap<_, _> = categories.into_iter().map(|c| (c.id, c)).collect();

    Ok(HttpResponseBuilder::new(StatusCode::OK)
        .content_type("application/xml")
        .append_header(actix_web::http::header::ContentDisposition {
            disposition: actix_web::http::header::DispositionType::Inline,
            parameters: vec![DispositionParam::Filename(String::from("kueaplan.xml"))],
        })
        .body(generate_frab_xml(
            event,
            entries,
            rooms,
            &categories_by_id,
            url_for_event,
            url_for_entry,
        )?))
}

#[derive(Deserialize, Serialize)]
pub struct FrabXmlQueryParams {
    #[serde(rename = "token")]
    pub session_token: String,
}

fn generate_frab_xml<F, G>(
    event: ExtendedEvent,
    entries: Vec<FullEntry>,
    rooms: Vec<Room>,
    categories: &BTreeMap<CategoryId, Category>,
    url_for_event: F,
    url_for_entry: G,
) -> Result<String, AppError>
where
    F: Fn(&EventId) -> String,
    G: Fn(&FullEntry, &ExtendedEvent) -> String,
{
    let rooms_by_id: BTreeMap<_, _> = rooms.iter().map(|r| (r.id, r)).collect();
    let grouped_entries = group_entries_by_date_and_room(&entries, &rooms_by_id, &event.clock_info);
    let data = Schedule {
        version: chrono::Utc::now().to_string(),
        generator: XmlGenerator {
            name: "online-kueaplan".to_string(),
            version: crate::get_version().to_string(),
        },
        conference: ConferenceMetaData::from_event_and_categories(
            &event,
            categories,
            url_for_event,
        ),
        day: grouped_entries
            .iter()
            .enumerate()
            .map(|(index, (date, rooms))| DaySchedule {
                index: (index + 1) as u32,
                date: *date,
                start: date
                    .and_time(event.clock_info.effective_begin_of_day)
                    .and_utc(),
                end: (*date + chrono::TimeDelta::days(1))
                    .and_time(event.clock_info.effective_begin_of_day)
                    .and_utc(),
                room: rooms
                    .iter()
                    .flat_map(|(room, entries)| {
                        if entries.is_empty() {
                            None
                        } else {
                            Some(RoomSchedule {
                                guid: if let Some(room) = room {
                                    room.id
                                } else {
                                    uuid::Uuid::default()
                                },
                                room_name: if let Some(room) = room {
                                    &room.title
                                } else {
                                    "[Ohne Ort]"
                                },
                                event: entries
                                    .iter()
                                    .map(|e| {
                                        XmlEntry::from_full_entry(
                                            e,
                                            categories,
                                            &rooms_by_id,
                                            |entry| url_for_entry(entry, &event),
                                        )
                                    })
                                    .collect(),
                            })
                        }
                    })
                    .collect(),
            })
            .collect(),
    };
    serde_xml_rs::to_string(&data).map_err(|e| AppError::InternalError(e.to_string()))
}

#[derive(Serialize)]
#[serde(rename = "schedule")]
struct Schedule<'a> {
    version: String,
    generator: XmlGenerator,
    conference: ConferenceMetaData<'a>,
    day: Vec<DaySchedule<'a>>,
}
#[derive(Serialize)]
struct ConferenceMetaData<'a> {
    title: &'a str,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
    days: i64,
    time_zone_name: &'a str,
    url: String,
    base_url: String,
    track: Vec<TrackMetaData<'a>>,
}
#[derive(Serialize)]
struct XmlGenerator {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@version")]
    version: String,
}

impl<'a> ConferenceMetaData<'a> {
    fn from_event_and_categories<F>(
        event: &'a ExtendedEvent,
        categories: &'a BTreeMap<CategoryId, Category>,
        url_for_event: F,
    ) -> Self
    where
        F: Fn(&EventId) -> String,
    {
        Self {
            title: &event.basic_data.title,
            start: event
                .basic_data
                .begin_date
                .and_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                .and_utc(),
            end: event
                .basic_data
                .end_date
                .and_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                .and_utc(),
            days: (event.basic_data.end_date - event.basic_data.begin_date).num_days() + 1,
            time_zone_name: event.clock_info.timezone.name(),
            url: url_for_event(&event.basic_data.id),
            base_url: url_for_event(&event.basic_data.id),
            track: categories
                .values()
                .map(|category| TrackMetaData {
                    name: &category.title,
                    color: format!("#{}", category.color.clone()),
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct TrackMetaData<'a> {
    #[serde(rename = "@name")]
    name: &'a str,
    #[serde(rename = "@color")]
    color: String,
}
#[derive(Serialize)]
struct DaySchedule<'a> {
    #[serde(rename = "@index")]
    index: u32,
    #[serde(rename = "@date")]
    date: chrono::NaiveDate,
    #[serde(rename = "@start")]
    start: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "@end")]
    end: chrono::DateTime<chrono::Utc>,
    room: Vec<RoomSchedule<'a>>,
}
#[derive(Serialize)]
struct RoomSchedule<'a> {
    #[serde(rename = "@guid")]
    guid: uuid::Uuid,
    #[serde(rename = "@name")]
    room_name: &'a str,
    event: Vec<XmlEntry<'a>>,
}

#[derive(Serialize)]
struct XmlEntry<'a> {
    #[serde(rename = "@id")]
    id: u32,
    #[serde(rename = "@guid")]
    guid: uuid::Uuid,
    date: chrono::DateTime<chrono::Utc>,
    start: chrono::NaiveTime,
    duration: String,
    room: String,
    url: String,
    title: &'a str,
    slug: &'a str,
    subtitle: &'a str,
    track: &'a str,
    //language: &'a str,
    #[serde(rename = "type")]
    type_: &'a str,
    #[serde(rename = "abstract")]
    abstract_: &'a str,
    description: String,
    //logo: &'a str,
    //links: XmlLinks,
    persons: XmlPersons<'a>,
    //attachments: XmlAttachments,
}
impl<'a> XmlEntry<'a> {
    fn from_full_entry<G>(
        entry: &'a FullEntry,
        categories: &'a BTreeMap<CategoryId, Category>,
        rooms_by_id: &'a BTreeMap<RoomId, &Room>,
        url_for_entry: G,
    ) -> Self
    where
        G: Fn(&FullEntry) -> String,
    {
        Self {
            id: simplehash::murmurhash3_32(entry.entry.id.as_bytes(), 0),
            guid: entry.entry.id,
            date: entry.entry.begin,
            start: entry.entry.begin.time(),
            duration: format_duration(entry.entry.end.signed_duration_since(entry.entry.begin)),
            room: generate_xml_entry_room(entry, rooms_by_id),
            url: url_for_entry(entry),
            title: &entry.entry.title,
            slug: "",
            subtitle: &entry.entry.comment,
            track: categories
                .get(&entry.entry.category)
                .map(|c| c.title.as_ref())
                .unwrap_or(""),
            //language: "",
            type_: "",
            abstract_: "",
            description: generate_xml_description(entry),
            //logo: "",
            //links: Default::default(),
            persons: XmlPersons::from_responsible_person(&entry.entry.responsible_person),
            //attachments: Default::default(),
        }
    }
}

#[derive(Serialize)]
struct XmlPersons<'a> {
    person: Vec<XmlPerson<'a>>,
}

impl<'a> XmlPersons<'a> {
    fn from_responsible_person(entry_persons: &'a str) -> Self {
        Self {
            person: if entry_persons.is_empty() {
                vec![]
            } else {
                vec![XmlPerson {
                    name: entry_persons,
                }]
            },
        }
    }
}

#[derive(Serialize)]
struct XmlPerson<'a> {
    #[serde(rename = "#text")]
    name: &'a str,
}
// #[derive(Default, Serialize)]
// struct XmlLinks;
// #[derive(Default, Serialize)]
// struct XmlAttachments;

#[allow(clippy::type_complexity)]
fn group_entries_by_date_and_room<'a>(
    entries: &'a Vec<FullEntry>,
    rooms_by_id: &'a BTreeMap<RoomId, &'a Room>,
    clock_info: &'a EventClockInfo,
) -> Vec<(
    chrono::NaiveDate,
    Vec<(Option<&'a Room>, Vec<&'a FullEntry>)>,
)> {
    let mut result = Vec::new();
    if entries.is_empty() {
        return result;
    }
    let mut block_entries: BTreeMap<Option<uuid::Uuid>, Vec<&FullEntry>> = BTreeMap::new();
    let mut current_date = get_effective_date(&entries[0].entry.begin, clock_info);
    for entry in entries {
        if entry.entry.is_cancelled {
            continue;
        }
        if get_effective_date(&entry.entry.begin, clock_info) != current_date {
            if !block_entries.is_empty() {
                result.push((
                    current_date,
                    finalize_room_block(block_entries, rooms_by_id),
                ));
            }
            block_entries = BTreeMap::new();
            current_date = get_effective_date(&entry.entry.begin, clock_info);
        }
        if entry.room_ids.is_empty() {
            block_entries
                .entry(None)
                .and_modify(|room_entries: &mut Vec<&FullEntry>| room_entries.push(entry))
                .or_insert(vec![entry]);
        }
        for room_id in entry.room_ids.iter() {
            block_entries
                .entry(Some(*room_id))
                .and_modify(|room_entries: &mut Vec<&FullEntry>| room_entries.push(entry))
                .or_insert(vec![entry]);
        }
    }
    if !block_entries.is_empty() {
        result.push((
            current_date,
            finalize_room_block(block_entries, rooms_by_id),
        ));
    }
    result
}

/// Uninlined code block from group_entries_by_date_and_room() for deduplication.
/// Transforms the intermediate block_entries map by room_uuid into the final result vector of
/// rooms + entries for a single date.
fn finalize_room_block<'a>(
    block_entries: BTreeMap<Option<uuid::Uuid>, Vec<&'a FullEntry>>,
    rooms_by_id: &BTreeMap<RoomId, &'a Room>,
) -> Vec<(Option<&'a Room>, Vec<&'a FullEntry>)> {
    block_entries
        .into_iter()
        .map(|(room_id, room_entries)| {
            (
                room_id.and_then(|rid| rooms_by_id.get(&rid).copied()),
                room_entries,
            )
        })
        .collect()
}

fn format_duration(duration: chrono::TimeDelta) -> String {
    let seconds = duration.num_seconds();
    format!(
        "{:0>2}:{:0>2}:{:0>2}",
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60
    )
}

/// Generate the <description> content for a given entry in the Frab XML format
fn generate_xml_description(entry: &FullEntry) -> String {
    let mut description = String::new();
    append_if_not_empty(&mut description, &entry.entry.time_comment, "\n");
    append_if_not_empty(&mut description, &entry.entry.room_comment, "\n");
    append_if_not_empty(&mut description, &entry.entry.description, "\n\n");
    description
}

/// Generate the <room> field for a given entry in the Frab XML format
fn generate_xml_entry_room(entry: &FullEntry, rooms: &BTreeMap<RoomId, &Room>) -> String {
    let room_names: Vec<String> = entry
        .room_ids
        .iter()
        .filter_map(|room_id| rooms.get(room_id))
        .map(|r| r.title.clone())
        .collect();

    let mut location = room_names.join(", ");
    if !entry.entry.room_comment.is_empty() {
        if !location.is_empty() {
            location.push_str("; ");
        }
        location.push_str(&entry.entry.room_comment);
    }

    location
}

pub fn xml_entry_url(
    req: &HttpRequest,
    event_id: EventId,
    entry_id: &EntryId,
    entry_begin_effective_date: &chrono::NaiveDate,
) -> Result<url::Url, UrlGenerationError> {
    let mut url = req.url_for(
        "main_list",
        [
            &event_id.to_string(),
            &entry_begin_effective_date.to_string(),
        ],
    )?;
    url.set_fragment(Some(&format!("entry-{}", entry_id)));
    Ok(url)
}

/// Utility function to append the string slice `source` to the `target` string, separated by
/// `separator` if neither of both is empty.
fn append_if_not_empty(target: &mut String, source: &str, separator: &str) {
    if source.is_empty() {
        return;
    }
    if !target.is_empty() {
        target.push_str(separator);
    }
    target.push_str(source);
}
