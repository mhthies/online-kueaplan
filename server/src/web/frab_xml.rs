use crate::auth_session::SessionToken;
use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, Event, FullEntry, Room};
use crate::data_store::{CategoryId, EntryFilter, RoomId};
use crate::web::time_calculation::{EFFECTIVE_BEGIN_OF_DAY, TIME_ZONE};
use crate::web::ui::error::AppError;
use crate::web::AppState;
use actix_web::http::header::DispositionParam;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpRequest, HttpResponseBuilder, Responder};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::format;

#[allow(clippy::identity_op)] // We want to explicitly state that it's "1" year
pub const SESSION_COOKIE_MAX_AGE: std::time::Duration =
    std::time::Duration::from_secs(1 * 86400 * 365);

#[get("/events/{event_id}/frab-xml")]
async fn frab_xml(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    query: web::Query<FrabXmlQueryParams>,
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

    Ok(HttpResponseBuilder::new(StatusCode::OK)
        .content_type("application/xml")
        .append_header(actix_web::http::header::ContentDisposition {
            disposition: actix_web::http::header::DispositionType::Inline,
            parameters: vec![DispositionParam::Filename(String::from("kueaplan.xml"))],
        })
        .body(generate_frab_xml(event, entries, rooms, categories)))
}

#[derive(Deserialize, Serialize)]
pub struct FrabXmlQueryParams {
    #[serde(rename = "token")]
    pub session_token: String,
}

fn generate_frab_xml(
    event: Event,
    entries: Vec<FullEntry>,
    rooms: Vec<Room>,
    categories: Vec<Category>,
) -> String {
    let grouped_entries: Vec<(chrono::NaiveDate, Vec<(&Room, Vec<&Category>)>)> = group_entries_by_date_and_room();
    let data = Schedule {
        version: chrono::DateTime::now().to_string(),
        conference: ConferenceMetaData {},
        day: grouped_entries.iter().enumerate().map(|(index, (date, rooms))| {
            DaySchedule {
                index,
                date,
                start: date.and_time(EFFECTIVE_BEGIN_OF_DAY).and_utc(),
                end: (date + chrono::TimeDelta::days(1)).and_time(EFFECTIVE_BEGIN_OF_DAY).and_utc(),
                room: rooms.iter().flat_map(|(room, entries)| if entries.is_empty() { None } else {
                    Some(RoomSchedule {
                        guid: room.id,
                        name: room.title,
                        event: entries.iter().map(),
                    }).collect(),
                },,
            }).collect(),
        };
    }
}

struct Schedule {
    version: String,
    conference: ConferenceMetaData,
    day: Vec<DaySchedule>,
}
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

impl ConferenceMetaData {
    fn from_event_and_categories(
        event: &Event,
        categories: Vec<Category>,
        request: &HttpRequest,
    ) -> Self {
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
            url: request
                .url_for("index", &[&event.id.to_string()])
                .unwrap()
                .to_string(),
            base_url: request
                .url_for("index", &[&event.id.to_string()])
                .unwrap()
                .to_string(),
            track: categories
                .iter()
                .map(|category| TrackMetaData {
                    name: category.title.clone(),
                    color: format!("#{}", category.color.clone()),
                })
                .collect(),
        }
    }
}

struct TrackMetaData {
    name: String,
    color: String,
}
struct DaySchedule {
    index: u32,
    date: chrono::NaiveDate,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
    room: Vec<RoomSchedule>,
}
struct RoomSchedule {
    guid: uuid::Uuid,
    name: String,
    event: Vec<XmlEntry>,
}

struct XmlEntry {
    guid: uuid::Uuid,
    date: chrono::DateTime<chrono::Utc>,
    start: chrono::NaiveTime,
    duration: chrono::Duration,
    room: String,
    url: String,
    title: String,
    slug: String,
    subtitle: String,
    track: String,
    language: String,
    type_: String,
    abstract_: String,
    description: String,
    logo: String,
    links: XmlLinks,
    persons: XmlPersons,
    attachments: XmlAttachments,
}
impl XmlEntry {
    fn from_
}

#[derive(Default)]
struct XmlPersons {
    person: Vec<XmlPerson>,
}
struct XmlPerson {
    name: String,
    guid: uuid::Uuid,
}
#[derive(Default)]
struct XmlLinks;
#[derive(Default)]
struct XmlAttachments;
