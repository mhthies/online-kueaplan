use crate::data_store::EntryFilter;
use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Helper function for deserializing a string, containing a comma-separated list of uuids, to a
/// `Vec<Uuid>` within a struct by deriving `serde::Deserialize` with
/// `#[serde(deserialize_with=...)]`.
pub fn deserialize_comma_separated_list_of_uuids<'de, D>(
    deserializer: D,
) -> Result<Vec<uuid::Uuid>, D::Error>
where
    D: Deserializer<'de>,
{
    let str_sequence = String::deserialize(deserializer)?;
    str_sequence
        .split(',')
        .filter(|s| !s.is_empty())
        .map(uuid::Uuid::parse_str)
        .collect::<Result<Vec<uuid::Uuid>, uuid::Error>>()
        .map_err(|_| {
            D::Error::invalid_value(
                Unexpected::Str(&str_sequence),
                &"A comma-separated list of uuids",
            )
        })
}

fn deserialize_optional_comma_separated_list_of_uuids<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<uuid::Uuid>>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(deserialize_comma_separated_list_of_uuids(
        deserializer,
    )?))
}

fn serialize_optional_comma_separated_list_of_uuids<S: Serializer>(
    value: &Option<Vec<uuid::Uuid>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if let Some(value) = value.as_ref() {
        let mut result = String::new();
        for (i, uuid) in value.iter().enumerate() {
            if i > 0 {
                result.push(',');
            }
            result.push_str(&uuid.to_string());
        }
        serializer.serialize_str(&result)
    } else {
        serializer.serialize_none()
    }
}

fn not(v: &bool) -> bool {
    !v
}

/// A struct that can be used as HTTP Query data on endpoints that return a list of KüA-Plan entries
/// to allow filtering the entries by time, category and room.
///
/// Typically, this struct should be used as type parameter for [actix_web::web::Query] as an
/// endpoint function parameter.
#[derive(Deserialize, Serialize, Default)]
pub struct EntryFilterAsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    after: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    before: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default, skip_serializing_if = "not")]
    after_exclusive: bool,
    #[serde(default, skip_serializing_if = "not")]
    before_inclusive: bool,
    #[serde(default, skip_serializing_if = "not")]
    match_previous_dates: bool,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_comma_separated_list_of_uuids",
        serialize_with = "serialize_optional_comma_separated_list_of_uuids"
    )]
    categories: Option<Vec<uuid::Uuid>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_comma_separated_list_of_uuids",
        serialize_with = "serialize_optional_comma_separated_list_of_uuids"
    )]
    rooms: Option<Vec<uuid::Uuid>>,
    #[serde(default, skip_serializing_if = "not")]
    without_room: bool,
}

impl From<EntryFilterAsQuery> for EntryFilter {
    fn from(value: EntryFilterAsQuery) -> Self {
        EntryFilter {
            after: value.after,
            after_inclusive: !value.after_exclusive,
            before: value.before,
            before_inclusive: value.before_inclusive,
            include_previous_date_matches: value.match_previous_dates,
            categories: value.categories,
            rooms: value.rooms,
            no_room: value.without_room,
        }
    }
}
