use chrono::{DateTime, Utc};
use serde::{self, Deserialize, Deserializer, Serializer};

/// Serialize a `DateTime<Utc>` as an RFC 3339 string.
///
/// # Errors
///
/// Returns the serializer error if writing the timestamp fails.
pub fn serialize<S>(value: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_rfc3339())
}

#[allow(dead_code)]
/// Deserialize an RFC 3339 string into a `DateTime<Utc>`.
///
/// # Errors
///
/// Returns the deserializer error if the input is missing or malformed.
pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}

pub mod option {
    use super::{DateTime, Deserialize, Deserializer, Serializer, Utc, serde};

    /// Serialize an optional `DateTime<Utc>` as RFC 3339 when present.
    ///
    /// # Errors
    ///
    /// Returns the serializer error if writing the value fails.
    pub fn serialize<S>(value: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(dt) => serializer.serialize_some(&dt.to_rfc3339()),
            None => serializer.serialize_none(),
        }
    }

    #[allow(dead_code)]
    /// Deserialize an optional RFC 3339 timestamp.
    ///
    /// # Errors
    ///
    /// Returns the deserializer error if the timestamp format is invalid.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        opt.map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(serde::de::Error::custom)
        })
        .transpose()
    }
}
