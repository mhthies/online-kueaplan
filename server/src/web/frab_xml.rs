use crate::auth_session::SessionToken;
use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, Event, FullEntry, Room};
use crate::data_store::{CategoryId, EntryFilter, EventId};
use crate::web::time_calculation::{get_effective_date, EFFECTIVE_BEGIN_OF_DAY, TIME_ZONE};
use crate::web::ui::error::AppError;
use crate::web::AppState;
use actix_web::http::header::DispositionParam;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpRequest, HttpResponseBuilder, Responder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
            .url_for("event_index", &[&event_id.to_string()])
            .unwrap()
            .to_string()
    };

    let (event, entries, rooms, categories) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok((
            store.get_event(event_id)?,
            store.get_entries_filtered(&auth, event_id, EntryFilter::default())?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
        ))
    })
    .await??;
    let categories_by_id: HashMap<_, _> = categories.into_iter().map(|c| (c.id, c)).collect();

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
        )))
}

#[derive(Deserialize, Serialize)]
pub struct FrabXmlQueryParams {
    #[serde(rename = "token")]
    pub session_token: String,
}

fn generate_frab_xml<F>(
    event: Event,
    entries: Vec<FullEntry>,
    rooms: Vec<Room>,
    categories: &HashMap<CategoryId, Category>,
    url_for_event: F,
) -> String
where
    F: Fn(&EventId) -> String,
{
    let grouped_entries = group_entries_by_date_and_room(&entries, &rooms);
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
                date: date.clone(),
                start: date.and_time(EFFECTIVE_BEGIN_OF_DAY).and_utc(),
                end: (*date + chrono::TimeDelta::days(1))
                    .and_time(EFFECTIVE_BEGIN_OF_DAY)
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
                                name: if let Some(room) = room {
                                    room.title.clone()
                                } else {
                                    "Ort nicht definiert".to_string()
                                },
                                event: entries
                                    .iter()
                                    .map(|e| XmlEntry::from_full_entry(e, &categories))
                                    .collect(),
                            })
                        }
                    })
                    .collect(),
            })
            .collect(),
    };
    serde_xml_rs::to_string(&data).unwrap() // TODO error handling
}

// TODO change structs to use references instead of owning datatypes
#[derive(Serialize)]
struct Schedule {
    version: String,
    generator: XmlGenerator,
    conference: ConferenceMetaData,
    day: Vec<DaySchedule>,
}
#[derive(Serialize)]
struct ConferenceMetaData {
    title: String,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
    days: i64,
    time_zone_name: String,
    url: String,
    base_url: String,
    track: Vec<TrackMetaData>,
}
#[derive(Serialize)]
struct XmlGenerator {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@version")]
    version: String,
}

impl ConferenceMetaData {
    fn from_event_and_categories<F>(
        event: &Event,
        categories: &HashMap<CategoryId, Category>,
        url_for_event: F,
    ) -> Self
    where
        F: Fn(&EventId) -> String,
    {
        Self {
            title: event.title.clone(),
            start: event
                .begin_date
                .and_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                .and_utc(),
            end: event
                .end_date
                .and_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                .and_utc(),
            days: (event.end_date - event.begin_date).num_days() + 1,
            time_zone_name: TIME_ZONE.name().to_owned(),
            url: url_for_event(&event.id),
            base_url: url_for_event(&event.id),
            track: categories
                .values()
                .map(|category| TrackMetaData {
                    name: category.title.clone(),
                    color: format!("#{}", category.color.clone()),
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct TrackMetaData {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@color")]
    color: String,
}
#[derive(Serialize)]
struct DaySchedule {
    #[serde(rename = "@index")]
    index: u32,
    #[serde(rename = "@date")]
    date: chrono::NaiveDate,
    #[serde(rename = "@start")]
    start: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "@end")]
    end: chrono::DateTime<chrono::Utc>,
    room: Vec<RoomSchedule>,
}
#[derive(Serialize)]
struct RoomSchedule {
    #[serde(rename = "@guid")]
    guid: uuid::Uuid,
    #[serde(rename = "@name")]
    name: String,
    event: Vec<XmlEntry>,
}

#[derive(Serialize)]
struct XmlEntry {
    #[serde(rename = "@guid")]
    guid: uuid::Uuid,
    date: chrono::DateTime<chrono::Utc>,
    start: chrono::NaiveTime,
    duration: String,
    room: String,
    url: String,
    title: String,
    slug: String,
    subtitle: String,
    track: String,
    language: String,
    #[serde(rename = "type")]
    type_: String,
    #[serde(rename = "abstract")]
    abstract_: String,
    description: String,
    logo: String,
    links: XmlLinks,
    persons: XmlPersons,
    attachments: XmlAttachments,
}
impl XmlEntry {
    fn from_full_entry(entry: &FullEntry, categories: &HashMap<CategoryId, Category>) -> Self {
        // TODO time_comment, room_comment
        // TODO room (which one(s)?)
        // TODO URL
        Self {
            guid: entry.entry.id,
            date: entry.entry.begin,
            start: entry.entry.begin.time(),
            duration: format_duration(entry.entry.end.signed_duration_since(entry.entry.begin)),
            room: "".to_string(),
            url: "".to_string(),
            title: entry.entry.title.clone(),
            slug: "".to_string(),
            subtitle: entry.entry.comment.clone(),
            track: categories
                .get(&entry.entry.category)
                .map(|c| c.title.clone())
                .unwrap_or("".to_string()),
            language: "".to_string(),
            type_: "".to_string(),
            abstract_: "".to_string(),
            description: entry.entry.description.clone(),
            logo: "".to_string(),
            links: Default::default(),
            persons: XmlPersons::from_responsible_person(entry.entry.responsible_person.clone()),
            attachments: Default::default(),
        }
    }
}

#[derive(Serialize)]
struct XmlPersons {
    person: Vec<XmlPerson>,
}

impl XmlPersons {
    fn from_responsible_person(entry_persons: String) -> Self {
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
struct XmlPerson {
    #[serde(rename = "#text")]
    name: String,
}
#[derive(Default, Serialize)]
struct XmlLinks;
#[derive(Default, Serialize)]
struct XmlAttachments;

fn group_entries_by_date_and_room<'a>(
    entries: &'a Vec<FullEntry>,
    rooms: &'a Vec<Room>,
) -> Vec<(
    chrono::NaiveDate,
    Vec<(Option<&'a Room>, Vec<&'a FullEntry>)>,
)> {
    let rooms_by_id: HashMap<_, _> = rooms.iter().map(|r| (r.id, r)).collect();
    let mut result = Vec::new();
    if entries.is_empty() {
        return result;
    }
    let mut block_entries: HashMap<Option<uuid::Uuid>, Vec<&FullEntry>> = HashMap::new();
    let mut current_date = get_effective_date(&entries[0].entry.begin);
    for entry in entries {
        if entry.entry.is_cancelled {
            continue;
        }
        if get_effective_date(&entry.entry.begin) != current_date {
            // TODO deduplicate code
            if !block_entries.is_empty() {
                result.push((
                    current_date,
                    block_entries
                        .into_iter()
                        .map(|(room_id, room_entries)| {
                            (
                                room_id.and_then(|rid| rooms_by_id.get(&rid).map(|r| *r)),
                                room_entries,
                            )
                        })
                        .collect(),
                ));
            }
            block_entries = HashMap::new();
            current_date = get_effective_date(&entry.entry.begin);
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
            block_entries
                .into_iter()
                .map(|(room_id, room_entries)| {
                    (
                        room_id.and_then(|rid| rooms_by_id.get(&rid).map(|r| *r)),
                        room_entries,
                    )
                })
                .collect(),
        ));
    }
    result
}

fn format_duration(duration: chrono::TimeDelta) -> String {
    let minutes = duration.num_minutes();
    format!("{:0>2}:{:0>2}", minutes / 60, minutes % 60)
}
