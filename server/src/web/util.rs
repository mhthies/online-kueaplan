use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer};

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
