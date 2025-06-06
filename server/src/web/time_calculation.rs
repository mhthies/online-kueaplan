use crate::data_store::models::Event;
use chrono::{DateTime, NaiveDate, TimeZone, Timelike};

// TODO move configuration to database / event
// in local time
pub const EFFECTIVE_BEGIN_OF_DAY: chrono::NaiveTime =
    chrono::NaiveTime::from_hms_opt(5, 30, 0).unwrap();
pub const TIME_ZONE: chrono_tz::Tz = chrono_tz::Europe::Berlin;
// in local time
pub const TIME_BLOCKS: [(&str, Option<chrono::NaiveTime>); 4] = [
    ("vom Vortag", Some(EFFECTIVE_BEGIN_OF_DAY)),
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

/// Calculate a (common) UTC timestamp from an effective date (i.e. using EFFECTIVE_BEGIN_OF_DAY
/// instead of 0:00 as begin of day) and a local time.
///
/// For example, with EFFECTIVE_BEGIN_OF_DAY = 05:30 and localtime = UTC+2:
/// * effective_date=2025-08-13, local_time=06:00 => 2025-08-13T04:00:00
/// * effective_date=2025-08-13, local_time=17:00 => 2025-08-13T15:00:00
/// * effective_date=2025-08-13, local_time=03:00 => 2025-08-14T01:00:00
pub fn timestamp_from_effective_date_and_time(
    effective_date: NaiveDate,
    local_time: chrono::NaiveTime,
) -> DateTime<chrono::Utc> {
    let date = effective_date
        + if local_time < EFFECTIVE_BEGIN_OF_DAY {
            chrono::Duration::days(1)
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

/// Get the current (effective) date, but clamp it to the event's boundaries
pub fn current_effective_date() -> chrono::NaiveDate {
    let now = chrono::Utc::now().with_timezone(&TIME_ZONE);
    now.date_naive()
        + if now.naive_local().time() < EFFECTIVE_BEGIN_OF_DAY {
            chrono::Duration::days(-1)
        } else {
            chrono::Duration::days(0)
        }
}

/// Calculate the most reasonable date to show the KüA-Plan for. Use the current (effective) date,
/// but clamp it to the event's boundaries
pub fn most_reasonable_date(event: &Event) -> chrono::NaiveDate {
    current_effective_date().clamp(event.begin_date, event.end_date)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_timestamp_from_effective_date_and_time() {
        assert_eq!(
            timestamp_from_effective_date_and_time(
                "2025-08-13".parse().unwrap(),
                "06:00".parse().unwrap()
            ),
            "2025-08-13T04:00:00+00:00"
                .parse::<chrono::DateTime<chrono::Utc>>()
                .unwrap()
        );
        assert_eq!(
            timestamp_from_effective_date_and_time(
                "2025-08-13".parse().unwrap(),
                "17:00".parse().unwrap()
            ),
            "2025-08-13T15:00:00+00:00"
                .parse::<chrono::DateTime<chrono::Utc>>()
                .unwrap()
        );
        assert_eq!(
            timestamp_from_effective_date_and_time(
                "2025-08-13".parse().unwrap(),
                "03:00".parse().unwrap()
            ),
            "2025-08-14T01:00:00+00:00"
                .parse::<chrono::DateTime<chrono::Utc>>()
                .unwrap()
        );
    }

    #[test]
    fn test_get_effective_date() {
        assert_eq!(
            get_effective_date(
                &"2025-08-13T04:00:00+00:00"
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap()
            ),
            "2025-08-13".parse().unwrap(),
        );
        assert_eq!(
            get_effective_date(
                &"2025-08-13T15:00:00+00:00"
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap()
            ),
            "2025-08-13".parse().unwrap(),
        );
        assert_eq!(
            get_effective_date(
                &"2025-08-14T01:00:00+00:00"
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap()
            ),
            "2025-08-13".parse().unwrap(),
        );
    }
}
