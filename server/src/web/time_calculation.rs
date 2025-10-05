use crate::data_store::models::{EventClockInfo, ExtendedEvent};
use chrono::{DateTime, NaiveDate, TimeZone, Timelike};

/// Calculate the effective date of a timestamp, considering the EFFECTIVE_BEGIN_OF_DAY (in local
/// time) instead of 0:00 as date boundary
pub fn get_effective_date(
    date_time: &DateTime<chrono::Utc>,
    clock_info: &EventClockInfo,
) -> chrono::NaiveDate {
    (date_time.with_timezone(&clock_info.timezone)
        - chrono::Duration::seconds(
            clock_info
                .effective_begin_of_day
                .num_seconds_from_midnight() as i64,
        ))
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
    clock_info: &EventClockInfo,
) -> DateTime<chrono::Utc> {
    let date = effective_date
        + if local_time < clock_info.effective_begin_of_day {
            chrono::Duration::days(1)
        } else {
            chrono::Duration::days(0)
        };
    let local_datetime = chrono::NaiveDateTime::new(date, local_time);
    clock_info
        .timezone
        .from_local_datetime(&local_datetime)
        .latest()
        .map(|dt| dt.to_utc())
        .unwrap_or(local_datetime.and_utc())
}

/// Get the current (effective) date, but clamp it to the event's boundaries
pub fn current_effective_date(clock_info: &EventClockInfo) -> chrono::NaiveDate {
    let now = chrono::Utc::now().with_timezone(&clock_info.timezone);
    now.date_naive()
        + if now.naive_local().time() < clock_info.effective_begin_of_day {
            chrono::Duration::days(-1)
        } else {
            chrono::Duration::days(0)
        }
}

/// Calculate the most reasonable date to show the KüA-Plan for. Use the current (effective) date,
/// but clamp it to the event's boundaries
pub fn most_reasonable_date(event: &ExtendedEvent) -> chrono::NaiveDate {
    current_effective_date(&event.clock_info)
        .clamp(event.basic_data.begin_date, event.basic_data.end_date)
}

#[cfg(test)]
mod tests {
    use super::*;
    const DEFAULT_CLOCK_INFO: EventClockInfo = EventClockInfo {
        timezone: chrono_tz::Tz::Europe__Berlin,
        effective_begin_of_day: chrono::NaiveTime::from_hms_opt(5, 30, 0).unwrap(),
    };

    #[test]
    fn test_timestamp_from_effective_date_and_time() {
        assert_eq!(
            timestamp_from_effective_date_and_time(
                "2025-08-13".parse().unwrap(),
                "06:00".parse().unwrap(),
                &DEFAULT_CLOCK_INFO
            ),
            "2025-08-13T04:00:00+00:00"
                .parse::<chrono::DateTime<chrono::Utc>>()
                .unwrap()
        );
        assert_eq!(
            timestamp_from_effective_date_and_time(
                "2025-08-13".parse().unwrap(),
                "17:00".parse().unwrap(),
                &DEFAULT_CLOCK_INFO
            ),
            "2025-08-13T15:00:00+00:00"
                .parse::<chrono::DateTime<chrono::Utc>>()
                .unwrap()
        );
        assert_eq!(
            timestamp_from_effective_date_and_time(
                "2025-08-13".parse().unwrap(),
                "03:00".parse().unwrap(),
                &DEFAULT_CLOCK_INFO
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
                    .unwrap(),
                &DEFAULT_CLOCK_INFO
            ),
            "2025-08-13".parse().unwrap(),
        );
        assert_eq!(
            get_effective_date(
                &"2025-08-13T15:00:00+00:00"
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap(),
                &DEFAULT_CLOCK_INFO
            ),
            "2025-08-13".parse().unwrap(),
        );
        assert_eq!(
            get_effective_date(
                &"2025-08-14T01:00:00+00:00"
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap(),
                &DEFAULT_CLOCK_INFO
            ),
            "2025-08-13".parse().unwrap(),
        );
    }
}
