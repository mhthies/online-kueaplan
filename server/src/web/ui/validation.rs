use crate::web::ui::forms::{
    FormValueRepresentation, ValidateFromFormInput, ValidationDataForFormValue,
};
use chrono::Timelike;
use lazy_static::lazy_static;
use uuid::Uuid;

#[derive(Default, Debug)]
pub struct NonEmptyString(pub String);

impl NonEmptyString {
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl FormValueRepresentation for NonEmptyString {
    fn into_form_value_string(self) -> String {
        self.0
    }
}
impl ValidateFromFormInput for NonEmptyString {
    fn from_form_value(value: &str) -> Result<Self, String> {
        if value.is_empty() {
            Err("Darf nicht leer sein".to_owned())
        } else {
            Ok(NonEmptyString(value.to_owned()))
        }
    }
}

#[derive(Default, Debug)]
pub struct UuidFromList(pub Uuid);

impl UuidFromList {
    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl FormValueRepresentation for UuidFromList {
    fn into_form_value_string(self) -> String {
        self.0.to_string()
    }
}

impl ValidationDataForFormValue<UuidFromList> for &Vec<Uuid> {
    fn validate_form_value(self, value: &'_ str) -> Result<UuidFromList, String> {
        let id = Uuid::parse_str(value).map_err(|e| e.to_string())?;
        if self.contains(&id) {
            Ok(UuidFromList(id))
        } else {
            Err("Unbekannte id".to_owned())
        }
    }
}

#[derive(Default, Debug)]
pub struct CommaSeparatedUuidsFromList(pub Vec<Uuid>);

impl CommaSeparatedUuidsFromList {
    pub fn into_inner(self) -> Vec<Uuid> {
        self.0
    }
}

impl FormValueRepresentation for CommaSeparatedUuidsFromList {
    fn into_form_value_string(self) -> String {
        self.0
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<String>>()
            .join(",")
    }
}

impl ValidationDataForFormValue<CommaSeparatedUuidsFromList> for &Vec<Uuid> {
    fn validate_form_value(self, value: &'_ str) -> Result<CommaSeparatedUuidsFromList, String> {
        let ids_str = value.split(',');
        let ids = ids_str
            .filter(|s| !s.is_empty())
            .map(|id_str| {
                let id = Uuid::parse_str(id_str).map_err(|e| e.to_string())?;
                if self.contains(&id) {
                    Ok(id)
                } else {
                    Err("Unbekannte id '{}'".to_owned())
                }
            })
            .collect::<Result<Vec<Uuid>, String>>()?;
        Ok(CommaSeparatedUuidsFromList(ids))
    }
}

#[derive(Default, Debug)]
pub struct TimeOfDay(pub chrono::NaiveTime);

impl TimeOfDay {
    pub fn into_inner(self) -> chrono::NaiveTime {
        self.0
    }
}

impl FormValueRepresentation for TimeOfDay {
    fn into_form_value_string(self) -> String {
        if self.0.second() != 0 || self.0.nanosecond() != 0 {
            self.0.format("%H:%M:%S%.f").to_string()
        } else {
            self.0.format("%H:%M").to_string()
        }
    }
}
impl ValidateFromFormInput for TimeOfDay {
    fn from_form_value(value: &str) -> Result<Self, String> {
        chrono::NaiveTime::parse_from_str(value, "%H:%M:%S%.f")
            .or_else(|_| chrono::NaiveTime::parse_from_str(value, "%H:%M"))
            .or_else(|_| chrono::NaiveTime::parse_from_str(value, "%H"))
            .map(Self)
            .map_err(|_| "Keine gültige Uhrzeit".to_owned())
    }
}

#[derive(Default, Debug)]
pub struct IsoDate(pub chrono::NaiveDate);

impl IsoDate {
    pub fn into_inner(self) -> chrono::NaiveDate {
        self.0
    }
}

impl FormValueRepresentation for IsoDate {
    fn into_form_value_string(self) -> String {
        self.0.format("%Y-%m-%d").to_string()
    }
}
impl ValidateFromFormInput for IsoDate {
    fn from_form_value(value: &str) -> Result<Self, String> {
        chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map(IsoDate)
            .map_err(|e| e.to_string())
    }
}

#[derive(Default, Debug)]
pub struct NiceDurationHours(pub chrono::Duration);

impl NiceDurationHours {
    pub fn into_inner(self) -> chrono::Duration {
        self.0
    }
}

impl FormValueRepresentation for NiceDurationHours {
    fn into_form_value_string(self) -> String {
        let days = self.0.num_days();
        let hours = self.0.num_hours() - 24 * self.0.num_days();
        let minutes = self.0.num_minutes() - 60 * self.0.num_hours();
        let seconds = self.0.num_seconds() - 60 * self.0.num_minutes();
        let milliseconds = self.0.subsec_nanos() / 1_000_000;

        let mut result = String::with_capacity(17);
        if days > 0 {
            result.push_str(&format!("{}d ", days));
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
impl ValidateFromFormInput for NiceDurationHours {
    fn from_form_value(value: &str) -> Result<Self, String> {
        lazy_static! {
            static ref RE: regex::Regex = regex::Regex::new(
                r"^(?:(?P<d>\d+)d\s*)?(?P<H>\d+)(?:[\.,](?P<Hf>\d{1,7}))?(?::(?P<M>\d+)(?:[\.,](?P<Mf>\d{1,5}))?(?::(?P<S>\d+)(?:[\.,](?P<Sf>\d{1,3}))?)?)?$").unwrap();
        }
        fn parse_group(cap: &regex::Captures, name: &str) -> Option<i64> {
            cap.name(name).map(|s| {
                s.as_str()
                    .parse::<i64>()
                    .expect("digits should be parseable as integer")
            })
        }
        fn parse_fraction_group(
            cap: &regex::Captures,
            name: &str,
            pad_right_to_length: usize,
            to_ms_nom: i64,
            to_ms_denom: i64,
        ) -> Option<i64> {
            cap.name(name)
                .map(|s| {
                    let padded = format!("{0:0<1$}", s.as_str(), pad_right_to_length);
                    padded
                        .parse::<i64>()
                        .expect("digits should be parseable as integer")
                })
                .map(|num| num * to_ms_nom / to_ms_denom)
        }

        RE.captures(value)
            .map(|cap| {
                let days = parse_group(&cap, "d").unwrap_or(0);
                let hours = parse_group(&cap, "H").unwrap_or(0);
                let hour_fraction_ms = parse_fraction_group(&cap, "Hf", 7, 9, 25).unwrap_or(0);
                let minutes = parse_group(&cap, "M").unwrap_or(0);
                let minute_fraction_ms = parse_fraction_group(&cap, "Mf", 5, 3, 5).unwrap_or(0);
                let seconds = parse_group(&cap, "S").unwrap_or(0);
                let milliseconds = parse_fraction_group(&cap, "Sf", 3, 1, 1).unwrap_or(0);

                Self(
                    chrono::Duration::days(days)
                        + chrono::Duration::hours(hours)
                        + chrono::Duration::milliseconds(hour_fraction_ms)
                        + chrono::Duration::minutes(minutes)
                        + chrono::Duration::milliseconds(minute_fraction_ms)
                        + chrono::Duration::seconds(seconds)
                        + chrono::Duration::milliseconds(milliseconds),
                )
            })
            .ok_or("Keine gültige Dauer".to_owned())
    }
}

#[derive(Default, Debug)]
pub struct SimpleTimestampMicroseconds(pub chrono::DateTime<chrono::Utc>);

impl FormValueRepresentation for SimpleTimestampMicroseconds {
    fn into_form_value_string(self) -> String {
        self.0.timestamp_micros().to_string()
    }
}
impl ValidateFromFormInput for SimpleTimestampMicroseconds {
    fn from_form_value(value: &str) -> Result<Self, String> {
        Ok(SimpleTimestampMicroseconds(
            chrono::DateTime::from_timestamp_micros(
                i64::from_str_radix(value, 10).map_err(|e| e.to_string())?,
            )
            .ok_or("Value out of range for chrono::DateTime".to_string())?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::uuid;

    fn get_example_uuids() -> Vec<Uuid> {
        vec![
            uuid!("165c1143-5a9c-4b2c-8548-d68658486763"),
            uuid!("b46b9e54-4316-4f07-a9d9-8b6323822467"),
            uuid!("21f253ea-a0c8-4f1e-a591-4a8000e979e9"),
        ]
    }

    #[test]
    fn test_comma_separated_uuids_from_list() {
        let result: CommaSeparatedUuidsFromList = get_example_uuids()
            .validate_form_value("21f253ea-a0c8-4f1e-a591-4a8000e979e9")
            .unwrap();
        assert_eq!(
            result.into_inner(),
            vec![uuid!("21f253ea-a0c8-4f1e-a591-4a8000e979e9")]
        );
        let result: CommaSeparatedUuidsFromList = get_example_uuids()
            .validate_form_value(
                "21f253ea-a0c8-4f1e-a591-4a8000e979e9,b46b9e54-4316-4f07-a9d9-8b6323822467",
            )
            .unwrap();
        assert_eq!(
            result.into_inner(),
            vec![
                uuid!("21f253ea-a0c8-4f1e-a591-4a8000e979e9"),
                uuid!("b46b9e54-4316-4f07-a9d9-8b6323822467")
            ]
        );
        let result: CommaSeparatedUuidsFromList =
            get_example_uuids().validate_form_value("").unwrap();
        assert_eq!(result.into_inner(), Vec::<Uuid>::new());
    }

    #[test]
    fn test_comma_separated_uuids_from_error() {
        let result: Result<CommaSeparatedUuidsFromList, _> =
            get_example_uuids().validate_form_value("21f253ea-a0c8-4f1e-a591-------------");
        assert!(result.is_err());
        let result: Result<CommaSeparatedUuidsFromList, _> = get_example_uuids()
            .validate_form_value(
                "21f253ea-a0c8-4f1e-a591-4a8000e979e9,9ab30a1f-f0b8-462d-ad4c-231f5ae214d6",
            );
        assert!(result.is_err());
    }

    #[test]
    fn test_nice_duration_hours_from_string() {
        assert_eq!(
            NiceDurationHours::from_form_value("2")
                .unwrap()
                .into_inner(),
            chrono::Duration::hours(2)
        );
        assert_eq!(
            NiceDurationHours::from_form_value("2:30")
                .unwrap()
                .into_inner(),
            chrono::Duration::hours(2) + chrono::Duration::minutes(30)
        );
        assert_eq!(
            NiceDurationHours::from_form_value("0:15")
                .unwrap()
                .into_inner(),
            chrono::Duration::minutes(15)
        );
        assert_eq!(
            NiceDurationHours::from_form_value("0:15:20")
                .unwrap()
                .into_inner(),
            chrono::Duration::minutes(15) + chrono::Duration::seconds(20)
        );
        assert_eq!(
            NiceDurationHours::from_form_value("2,5")
                .unwrap()
                .into_inner(),
            chrono::Duration::hours(2) + chrono::Duration::minutes(30)
        );
        assert_eq!(
            NiceDurationHours::from_form_value("1d 2")
                .unwrap()
                .into_inner(),
            chrono::Duration::days(1) + chrono::Duration::hours(2)
        );
        assert_eq!(
            NiceDurationHours::from_form_value("1:17,25")
                .unwrap()
                .into_inner(),
            chrono::Duration::hours(1)
                + chrono::Duration::minutes(17)
                + chrono::Duration::seconds(15)
        );
    }

    #[test]
    fn test_nice_duration_hours_roundtrip() {
        let val = chrono::Duration::hours(2);
        assert_eq!(
            val,
            NiceDurationHours::from_form_value(&NiceDurationHours(val).into_form_value_string())
                .unwrap()
                .into_inner()
        );
        let val = chrono::Duration::hours(2) + chrono::Duration::minutes(30);
        assert_eq!(
            val,
            NiceDurationHours::from_form_value(&NiceDurationHours(val).into_form_value_string())
                .unwrap()
                .into_inner()
        );
        let val = chrono::Duration::minutes(15);
        assert_eq!(
            val,
            NiceDurationHours::from_form_value(&NiceDurationHours(val).into_form_value_string())
                .unwrap()
                .into_inner()
        );
        let val = chrono::Duration::minutes(15) + chrono::Duration::seconds(20);
        assert_eq!(
            val,
            NiceDurationHours::from_form_value(&NiceDurationHours(val).into_form_value_string())
                .unwrap()
                .into_inner()
        );
        let val = chrono::Duration::hours(2) + chrono::Duration::minutes(30);
        assert_eq!(
            val,
            NiceDurationHours::from_form_value(&NiceDurationHours(val).into_form_value_string())
                .unwrap()
                .into_inner()
        );
        let val = chrono::Duration::days(1) + chrono::Duration::hours(2);
        assert_eq!(
            val,
            NiceDurationHours::from_form_value(&NiceDurationHours(val).into_form_value_string())
                .unwrap()
                .into_inner()
        );
        let val = chrono::Duration::hours(1)
            + chrono::Duration::minutes(17)
            + chrono::Duration::seconds(15);
        assert_eq!(
            val,
            NiceDurationHours::from_form_value(&NiceDurationHours(val).into_form_value_string())
                .unwrap()
                .into_inner()
        );
        let val = chrono::Duration::hours(1)
            + chrono::Duration::minutes(17)
            + chrono::Duration::seconds(15)
            + chrono::Duration::milliseconds(110);
        assert_eq!(
            val,
            NiceDurationHours::from_form_value(&NiceDurationHours(val).into_form_value_string())
                .unwrap()
                .into_inner()
        );
    }

    #[test]
    fn test_nice_duration_hours_errors() {
        assert!(NiceDurationHours::from_form_value("1:").is_err());
        assert!(NiceDurationHours::from_form_value("1:1:1:1").is_err());
        assert!(NiceDurationHours::from_form_value("5d 1d 1").is_err());
        assert!(NiceDurationHours::from_form_value("").is_err());
        assert!(NiceDurationHours::from_form_value("d").is_err());
        assert!(NiceDurationHours::from_form_value("1a").is_err());
        assert!(NiceDurationHours::from_form_value("abc5:5").is_err());
    }
}
