use crate::web::ui::forms::{FromFormValue, FromFormValueWithData, IntoFormValue};
use chrono::Timelike;
use lazy_static::lazy_static;
use uuid::Uuid;

pub struct NonEmptyString(pub String);

impl NonEmptyString {
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl FromFormValue<'_> for NonEmptyString {
    fn from_form_value(value: &'_ str) -> Result<Self, String> {
        if value.is_empty() {
            Err("Darf nicht leer sein".to_owned())
        } else {
            Ok(NonEmptyString(value.to_owned()))
        }
    }
}

pub struct UuidFromList(pub Uuid);

impl UuidFromList {
    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl<'d> FromFormValueWithData<'_, 'd> for UuidFromList {
    type AdditionalData = &'d Vec<Uuid>;

    fn from_form_value(value: &'_ str, known_ids: Self::AdditionalData) -> Result<Self, String> {
        let id = Uuid::parse_str(value).map_err(|e| e.to_string())?;
        if known_ids.contains(&id) {
            Ok(UuidFromList(id))
        } else {
            Err("Unbekannte id".to_owned())
        }
    }
}

pub struct CommaSeparatedUuidsFromList(pub Vec<Uuid>);

impl CommaSeparatedUuidsFromList {
    pub fn into_inner(self) -> Vec<Uuid> {
        self.0
    }
}

impl<'d> FromFormValueWithData<'_, 'd> for CommaSeparatedUuidsFromList {
    type AdditionalData = &'d Vec<Uuid>;

    fn from_form_value(value: &'_ str, known_ids: Self::AdditionalData) -> Result<Self, String> {
        let ids_str = value.split(',');
        let ids = ids_str
            .map(|id_str| {
                let id = Uuid::parse_str(id_str).map_err(|e| e.to_string())?;
                if known_ids.contains(&id) {
                    Ok(id)
                } else {
                    Err("Unbekannte id '{}'".to_owned())
                }
            })
            .collect::<Result<Vec<Uuid>, String>>()?;
        Ok(Self(ids))
    }
}

impl IntoFormValue for CommaSeparatedUuidsFromList {
    fn into_form_value_string(self) -> String {
        self.0
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<String>>()
            .join(",")
    }
}

pub struct TimeOfDay(pub chrono::NaiveTime);

impl TimeOfDay {
    pub fn into_inner(self) -> chrono::NaiveTime {
        self.0
    }
}

impl FromFormValue<'_> for TimeOfDay {
    fn from_form_value(value: &'_ str) -> Result<Self, String> {
        chrono::NaiveTime::parse_from_str(value, "%H:%M:%S%.f")
            .or_else(|_| chrono::NaiveTime::parse_from_str(value, "%H:%M"))
            .or_else(|_| chrono::NaiveTime::parse_from_str(value, "%H"))
            .map(Self)
            .map_err(|_| "Keine gültige Uhrzeit".to_owned())
    }
}

impl IntoFormValue for TimeOfDay {
    fn into_form_value_string(self) -> String {
        if self.0.second() != 0 || self.0.nanosecond() != 0 {
            self.0.format("%H:%M:%S%.f").to_string()
        } else {
            self.0.format("%H:%M").to_string()
        }
    }
}

pub struct IsoDate(pub chrono::NaiveDate);

impl IsoDate {
    pub fn into_inner(self) -> chrono::NaiveDate {
        self.0
    }
}

impl FromFormValue<'_> for IsoDate {
    fn from_form_value(value: &'_ str) -> Result<Self, String> {
        chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map(IsoDate)
            .map_err(|e| e.to_string())
    }
}

impl IntoFormValue for IsoDate {
    fn into_form_value_string(self) -> String {
        self.0.format("%Y-%m-%d").to_string()
    }
}

pub struct NiceDurationHours(pub chrono::Duration);

impl NiceDurationHours {
    pub fn into_inner(self) -> chrono::Duration {
        self.0
    }
}

impl FromFormValue<'_> for NiceDurationHours {
    fn from_form_value(value: &'_ str) -> Result<Self, String> {
        lazy_static! {
            static ref RE: regex::Regex = regex::Regex::new(
                r"(?:(?P<d>\d+)d )?(?:(?P<H2>\d+)h|(?:(?P<H>\d+):)?(?P<M>\d+)(?::(?P<S>\d+)(?:\.(?P<f>\d+))?)?)").unwrap();
        }
        fn parse_group(cap: &regex::Captures, name: &str) -> Option<i64> {
            cap.name(name).map(|s| {
                s.as_str()
                    .parse::<i64>()
                    .expect("digits should be parseable as integer")
            })
        }

        RE.captures(value)
            .map(|cap| {
                let days = parse_group(&cap, "d").unwrap_or(0);
                let hours = parse_group(&cap, "H")
                    .or(parse_group(&cap, "H2"))
                    .unwrap_or(0);
                let minutes = parse_group(&cap, "M").unwrap_or(0);
                let seconds = parse_group(&cap, "S").unwrap_or(0);
                let nanoseconds = cap
                    .name("f")
                    .map(|s| {
                        let padded = format!("{:0<9}", s.as_str());
                        padded
                            .parse::<i64>()
                            .expect("digits should be parseable as integer")
                    })
                    .unwrap_or(0);

                Self(
                    chrono::Duration::days(days)
                        + chrono::Duration::hours(hours)
                        + chrono::Duration::minutes(minutes)
                        + chrono::Duration::seconds(seconds)
                        + chrono::Duration::nanoseconds(nanoseconds),
                )
            })
            .ok_or("Keine gültige Dauer".to_owned())
    }
}

impl IntoFormValue for NiceDurationHours {
    fn into_form_value_string(self) -> String {
        let days = self.0.num_days();
        let hours = self.0.num_hours() - 24 * self.0.num_days();
        let minutes = self.0.num_minutes() - 60 * self.0.num_hours();
        let seconds = self.0.num_seconds() - 60 * self.0.num_minutes();
        let milliseconds = self.0.subsec_nanos() / 1_000_000;

        let mut result = String::with_capacity(17);
        if days > 0 {
            result.push_str(&format!("{}d", days));
        }
        result.push_str(&format!("{:02}:{:02}", hours, minutes));
        if seconds > 0 || milliseconds > 0 {
            result.push_str(&format!(":{:02}", seconds));
            if milliseconds > 0 {
                result.push_str(&format!(".{:03}", milliseconds));
            }
        }
        result
    }
}

pub struct SimpleTimestampMicroseconds(pub chrono::DateTime<chrono::Utc>);

impl FromFormValue<'_> for SimpleTimestampMicroseconds {
    fn from_form_value(value: &'_ str) -> Result<Self, String> {
        Ok(SimpleTimestampMicroseconds(
            chrono::DateTime::from_timestamp_micros(
                i64::from_str_radix(value, 10).map_err(|e| e.to_string())?,
            )
            .ok_or("Value out of range for chrono::DateTime".to_string())?,
        ))
    }
}

impl IntoFormValue for SimpleTimestampMicroseconds {
    fn into_form_value_string(self) -> String {
        self.0.timestamp_micros().to_string()
    }
}
