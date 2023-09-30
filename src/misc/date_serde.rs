use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Deserializer, Serializer};

// The signature of a serialize_with function must follow the pattern:
//
//    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
//    where
//        S: Serializer
//
// although it may also be generic over the input types T.
pub fn serialize<S>(
    date: &DateTime<Utc>,
    serializer: S,
) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
{
    serializer.serialize_str(&date.to_rfc3339_opts(SecondsFormat::Millis, true))
}

// The signature of a deserialize_with function must follow the pattern:
//
//    fn deserialize<'de, D>(D) -> Result<T, D::Error>
//    where
//        D: Deserializer<'de>
//
// although it may also be generic over the output types T.
pub fn deserialize<'de, D>(
    deserializer: D,
) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match DateTime::parse_from_rfc3339(&s) {
        Ok(date) => Ok(date.with_timezone(&Utc)),
        Err(e) => Err(serde::de::Error::custom(e)),
    }
}