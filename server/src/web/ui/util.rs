use crate::data_store::models::Event;
use crate::data_store::{EntryId, EventId};
use actix_web::error::UrlGenerationError;
use actix_web::HttpRequest;
use chrono::{DateTime, NaiveDate, TimeZone, Timelike};
use palette::{FromColor, IntoColor};

// TODO move configuration to database / event
// in local time
pub const EFFECTIVE_BEGIN_OF_DAY: chrono::NaiveTime =
    chrono::NaiveTime::from_hms_opt(5, 30, 0).unwrap();
pub const TIME_ZONE: chrono_tz::Tz = chrono_tz::Europe::Berlin;
// in local time
pub const TIME_BLOCKS: [(&str, Option<chrono::NaiveTime>); 3] = [
    (
        "Morgens",
        Some(chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap()),
    ),
    (
        "Mittags",
        Some(chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap()),
    ),
    ("Abends", None),
];

/// Calculate the effective date of a timestamp, considering the EFFECTIVE_BEGIN_OF_DAY (in local
/// time) instead of 0:00 as date boundary
pub fn get_effective_date(date_time: &DateTime<chrono::Utc>) -> chrono::NaiveDate {
    (date_time.with_timezone(&TIME_ZONE)
        - chrono::Duration::seconds(EFFECTIVE_BEGIN_OF_DAY.num_seconds_from_midnight() as i64))
    .date_naive()
}

pub fn timestamp_from_effective_date_and_time(
    effective_date: NaiveDate,
    local_time: chrono::NaiveTime,
) -> DateTime<chrono::Utc> {
    let date = effective_date
        + if local_time < EFFECTIVE_BEGIN_OF_DAY {
            chrono::Duration::days(-1)
        } else {
            chrono::Duration::days(0)
        };
    let local_datetime = chrono::NaiveDateTime::new(date, local_time);
    TIME_ZONE
        .from_local_datetime(&local_datetime)
        .latest()
        .map(|dt| dt.to_utc())
        .unwrap_or(local_datetime.and_utc())
}

/// Calculate the most reasonable date to show the KüA-Plan for. Use the current (effective) date,
/// but clamp it to the event's boundaries
pub fn most_reasonable_date(event: Event) -> chrono::NaiveDate {
    let now = chrono::Utc::now().with_timezone(&TIME_ZONE);
    let effective_date = now.date_naive()
        + if now.naive_local().time() < EFFECTIVE_BEGIN_OF_DAY {
            chrono::Duration::days(-1)
        } else {
            chrono::Duration::days(0)
        };
    effective_date.clamp(event.begin_date, event.end_date)
}

pub struct CategoryColors {
    background_light: palette::Srgb<u8>,
    border_light: palette::Srgb<u8>,
    text_light: palette::Srgb<u8>,
    background_dark: palette::Srgb<u8>,
    border_dark: palette::Srgb<u8>,
    text_dark: palette::Srgb<u8>,
}

impl CategoryColors {
    pub fn from_base_color_hex(base_color_hex: &str) -> Result<Self, String> {
        let base_color: palette::Srgb<u8> = base_color_hex.parse().map_err(|e| format!("{}", e))?;
        let base_color_hsl: palette::Hsl = base_color.into_format::<f32>().into_color();

        Ok(Self {
            background_light: palette::Srgb::<f32>::from_color(change_color_luminance(
                &base_color_hsl,
                0.925,
            ))
            .into_format(),
            border_light: palette::Srgb::<f32>::from_color(change_color_luminance(
                &base_color_hsl,
                0.7,
            ))
            .into_format(),
            text_light: palette::Srgb::<f32>::from_color(change_color_luminance(
                &base_color_hsl,
                0.2,
            ))
            .into_format(),
            background_dark: palette::Srgb::<f32>::from_color(change_color_luminance(
                &base_color_hsl,
                0.10,
            ))
            .into_format(),
            border_dark: palette::Srgb::<f32>::from_color(change_color_luminance(
                &base_color_hsl,
                0.3,
            ))
            .into_format(),
            text_dark: palette::Srgb::<f32>::from_color(change_color_luminance(
                &base_color_hsl,
                0.8,
            ))
            .into_format(),
        })
    }

    pub fn as_css(&self) -> String {
        format!(
            "--category-bg:#{:x};--category-text:#{:x};--category-border:#{:x};\
            --category-bg-dark:#{:x};--category-text-dark:#{:x};--category-border-dark:#{:x};",
            self.background_light,
            self.text_light,
            self.border_light,
            self.background_dark,
            self.text_dark,
            self.border_dark,
        )
    }
}

/// Change luminance to target value +- 0.1 (based on the original luminance) and reduce
/// saturation after large changes of luminance.
///
/// New base luminance must not be < 0.075 or > 0.925.
fn change_color_luminance(color: &palette::Hsl, new_base_luminance: f32) -> palette::Hsl {
    debug_assert!(new_base_luminance >= 0.075);
    debug_assert!(new_base_luminance <= 0.925);
    let target_luminance = new_base_luminance + color.lightness * 0.15 - 0.075;
    let luminance_difference = (target_luminance - color.lightness).abs();
    let saturation_factor = 1.0 - luminance_difference * 0.6;
    let mut color = *color;
    color.lightness = target_luminance;
    color.saturation *= saturation_factor;
    color
}

/// Calculate the list of calendar days that the event covers
pub fn event_days(event: &Event) -> Vec<chrono::NaiveDate> {
    let len = (event.end_date - event.begin_date).num_days();
    (0..=len)
        .map(|i| event.begin_date + chrono::Duration::days(i))
        .collect()
}

pub fn url_for_entry(
    req: &HttpRequest,
    event_id: EventId,
    entry_id: &EntryId,
    entry_begin: &chrono::DateTime<chrono::Utc>,
) -> Result<url::Url, UrlGenerationError> {
    let mut url = req.url_for(
        "main_list",
        [
            &event_id.to_string(),
            &get_effective_date(entry_begin).to_string(),
        ],
    )?;
    url.set_fragment(Some(&format!("entry-{}", entry_id)));
    Ok(url)
}
