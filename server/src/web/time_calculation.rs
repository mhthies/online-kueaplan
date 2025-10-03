use crate::data_store::models::ExtendedEvent;
use chrono::{DateTime, NaiveDate, TimeZone, Timelike};

/// Calculate the effective date of a timestamp, considering the EFFECTIVE_BEGIN_OF_DAY (in local
/// time) instead of 0:00 as date boundary
pub fn get_effective_date(
    date_time: &DateTime<chrono::Utc>,
    event: &ExtendedEvent,
) -> chrono::NaiveDate {
    (date_time.with_timezone(&event.timezone)
        - chrono::Duration::seconds(event.effective_begin_of_day.num_seconds_from_midnight() as i64))
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
    event: &ExtendedEvent,
) -> DateTime<chrono::Utc> {
    let date = effective_date
        + if local_time < event.effective_begin_of_day {
            chrono::Duration::days(1)
        } else {
            chrono::Duration::days(0)
        };
    let local_datetime = chrono::NaiveDateTime::new(date, local_time);
    event
        .timezone
        .from_local_datetime(&local_datetime)
        .latest()
        .map(|dt| dt.to_utc())
        .unwrap_or(local_datetime.and_utc())
}

/// Get the current (effective) date, but clamp it to the event's boundaries
pub fn current_effective_date(event: &ExtendedEvent) -> chrono::NaiveDate {
    let now = chrono::Utc::now().with_timezone(&event.timezone);
    now.date_naive()
        + if now.naive_local().time() < event.effective_begin_of_day {
            chrono::Duration::days(-1)
        } else {
            chrono::Duration::days(0)
        }
}

/// Calculate the most reasonable date to show the KüA-Plan for. Use the current (effective) date,
/// but clamp it to the event's boundaries
pub fn most_reasonable_date(event: &ExtendedEvent) -> chrono::NaiveDate {
    current_effective_date(event).clamp(event.basic_data.begin_date, event.basic_data.end_date)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_store::models::Event;
    fn get_test_event() -> ExtendedEvent {
        ExtendedEvent {
            basic_data: Event {
                id: 0,
                title: "Test".to_string(),
                begin_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 1, 5).unwrap(),
            },
            timezone: chrono_tz::Tz::Europe__Berlin,
            effective_begin_of_day: chrono::NaiveTime::from_hms_milli_opt(5, 30, 0, 0).unwrap(),
            default_time_schedule: crate::data_store::models::EventDayTimeSchedule {
                sections: vec![],
            },
        }
    }

    #[test]
    fn test_timestamp_from_effective_date_and_time() {
        assert_eq!(
            timestamp_from_effective_date_and_time(
                "2025-08-13".parse().unwrap(),
                "06:00".parse().unwrap(),
                &get_test_event()
            ),
            "2025-08-13T04:00:00+00:00"
                .parse::<chrono::DateTime<chrono::Utc>>()
                .unwrap()
        );
        assert_eq!(
            timestamp_from_effective_date_and_time(
                "2025-08-13".parse().unwrap(),
                "17:00".parse().unwrap(),
                &get_test_event()
            ),
            "2025-08-13T15:00:00+00:00"
                .parse::<chrono::DateTime<chrono::Utc>>()
                .unwrap()
        );
        assert_eq!(
            timestamp_from_effective_date_and_time(
                "2025-08-13".parse().unwrap(),
                "03:00".parse().unwrap(),
                &get_test_event()
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
                &get_test_event()
            ),
            "2025-08-13".parse().unwrap(),
        );
        assert_eq!(
            get_effective_date(
                &"2025-08-13T15:00:00+00:00"
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap(),
                &get_test_event()
            ),
            "2025-08-13".parse().unwrap(),
        );
        assert_eq!(
            get_effective_date(
                &"2025-08-14T01:00:00+00:00"
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap(),
                &get_test_event()
            ),
            "2025-08-13".parse().unwrap(),
        );
    }
}
