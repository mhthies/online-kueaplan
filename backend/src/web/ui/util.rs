use crate::data_store::models::Event;

// TODO move configuration to database / event
pub const EFFECTIVE_BEGIN_OF_DAY: chrono::NaiveTime =
    chrono::NaiveTime::from_hms_opt(5, 30, 0).unwrap();
pub const TIME_ZONE: chrono_tz::Tz = chrono_tz::Europe::Berlin;
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
