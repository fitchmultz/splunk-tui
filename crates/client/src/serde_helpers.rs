//! Serde helpers for Splunk's inconsistent JSON typing.
//!
//! Responsibilities:
//! - Provide deserializers that accept either JSON numbers or strings for numeric fields.
//! - Keep parsing behavior centralized so model definitions stay readable and consistent.
//!
//! Explicitly does NOT handle:
//! - Validating higher-level semantics (ranges, required/optional business rules).
//! - Normalizing units or performing domain conversions.
//!
//! Invariants / assumptions:
//! - Splunk may return numeric fields as `"123"` strings or as `123` numbers depending on endpoint/version.
//! - These helpers must not log or print secrets; errors should be generic parse errors.

use serde::Deserialize;
use serde::de::Error as _;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum U64OrString {
    U64(u64),
    I64(i64),
    String(String),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StringOrNumber {
    String(String),
    U64(u64),
    I64(i64),
    F64(f64),
}

#[allow(dead_code)]
pub fn u64_from_string_or_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = U64OrString::deserialize(deserializer)?;
    match value {
        U64OrString::U64(v) => Ok(v),
        U64OrString::I64(v) => u64::try_from(v).map_err(D::Error::custom),
        U64OrString::String(s) => s.parse::<u64>().map_err(D::Error::custom),
    }
}

#[allow(dead_code)]
pub fn opt_u64_from_string_or_number<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<U64OrString>::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(U64OrString::U64(v)) => Ok(Some(v)),
        Some(U64OrString::I64(v)) => Ok(Some(u64::try_from(v).map_err(D::Error::custom)?)),
        Some(U64OrString::String(s)) => Ok(Some(s.parse::<u64>().map_err(D::Error::custom)?)),
    }
}

pub fn usize_from_string_or_number<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = U64OrString::deserialize(deserializer)?;
    match value {
        U64OrString::U64(v) => Ok(v as usize),
        U64OrString::I64(v) => usize::try_from(v).map_err(D::Error::custom),
        U64OrString::String(s) => s.parse::<usize>().map_err(D::Error::custom),
    }
}

pub fn opt_usize_from_string_or_number<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<U64OrString>::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(U64OrString::U64(v)) => Ok(Some(v as usize)),
        Some(U64OrString::I64(v)) => Ok(Some(usize::try_from(v).map_err(D::Error::custom)?)),
        Some(U64OrString::String(s)) => Ok(Some(s.parse::<usize>().map_err(D::Error::custom)?)),
    }
}

pub fn opt_i32_from_string_or_number<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<StringOrNumber>::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(StringOrNumber::String(s)) => s.parse::<i32>().map_err(D::Error::custom).map(Some),
        Some(StringOrNumber::U64(v)) => i32::try_from(v).map_err(D::Error::custom).map(Some),
        Some(StringOrNumber::I64(v)) => i32::try_from(v).map_err(D::Error::custom).map(Some),
        Some(StringOrNumber::F64(v)) => Ok(Some(v as i32)),
    }
}

pub fn opt_string_from_number_or_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<StringOrNumber>::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(StringOrNumber::String(s)) => Ok(Some(s)),
        Some(StringOrNumber::U64(v)) => Ok(Some(v.to_string())),
        Some(StringOrNumber::I64(v)) => Ok(Some(v.to_string())),
        Some(StringOrNumber::F64(v)) => Ok(Some(v.to_string())),
    }
}

pub fn string_from_number_or_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = StringOrNumber::deserialize(deserializer)?;
    match value {
        StringOrNumber::String(s) => Ok(s),
        StringOrNumber::U64(v) => Ok(v.to_string()),
        StringOrNumber::I64(v) => Ok(v.to_string()),
        StringOrNumber::F64(v) => Ok(v.to_string()),
    }
}

#[allow(dead_code)]
pub fn map_string_to_u64_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = HashMap::<String, serde_json::Value>::deserialize(deserializer)?;
    let mut out = HashMap::with_capacity(raw.len());
    for (k, v) in raw {
        let parsed = v
            .as_u64()
            .or_else(|| v.as_i64().and_then(|i| u64::try_from(i).ok()))
            .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
            .ok_or_else(|| D::Error::custom("invalid map value for u64"))?;
        out.insert(k, parsed);
    }
    Ok(out)
}

pub fn map_string_to_usize_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = HashMap::<String, serde_json::Value>::deserialize(deserializer)?;
    let mut out = HashMap::with_capacity(raw.len());
    for (k, v) in raw {
        let parsed = v
            .as_u64()
            .or_else(|| v.as_i64().and_then(|i| u64::try_from(i).ok()))
            .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
            .map(|n| n as usize)
            .ok_or_else(|| D::Error::custom("invalid map value for usize"))?;
        out.insert(k, parsed);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u64_from_string_or_number_accepts_number() {
        #[derive(Deserialize)]
        struct Wrapper {
            #[serde(deserialize_with = "u64_from_string_or_number")]
            value: u64,
        }

        let parsed: Wrapper = serde_json::from_str(r#"{ "value": 3 }"#).unwrap();
        assert_eq!(parsed.value, 3);
    }

    #[test]
    fn test_u64_from_string_or_number_accepts_string() {
        #[derive(Deserialize)]
        struct Wrapper {
            #[serde(deserialize_with = "u64_from_string_or_number")]
            value: u64,
        }

        let parsed: Wrapper = serde_json::from_str(r#"{ "value": "3" }"#).unwrap();
        assert_eq!(parsed.value, 3);
    }

    #[test]
    fn test_opt_u64_from_string_or_number_accepts_null_and_missing() {
        #[derive(Deserialize)]
        struct Wrapper {
            #[serde(default, deserialize_with = "opt_u64_from_string_or_number")]
            value: Option<u64>,
        }

        let parsed: Wrapper = serde_json::from_str(r#"{ "value": null }"#).unwrap();
        assert_eq!(parsed.value, None);

        let parsed: Wrapper = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(parsed.value, None);
    }

    #[test]
    fn test_opt_string_from_number_or_string_accepts_number_and_string() {
        #[derive(Deserialize)]
        struct Wrapper {
            #[serde(default, deserialize_with = "opt_string_from_number_or_string")]
            value: Option<String>,
        }

        let parsed: Wrapper = serde_json::from_str(r#"{ "value": 3 }"#).unwrap();
        assert_eq!(parsed.value.as_deref(), Some("3"));

        let parsed: Wrapper = serde_json::from_str(r#"{ "value": "auto" }"#).unwrap();
        assert_eq!(parsed.value.as_deref(), Some("auto"));
    }

    #[test]
    fn test_string_from_number_or_string_accepts_number() {
        #[derive(Deserialize)]
        struct Wrapper {
            #[serde(deserialize_with = "string_from_number_or_string")]
            value: String,
        }

        let parsed: Wrapper = serde_json::from_str(r#"{ "value": 3 }"#).unwrap();
        assert_eq!(parsed.value, "3");
    }

    #[test]
    fn test_map_string_to_u64_from_string_or_number_accepts_strings() {
        #[derive(Deserialize)]
        struct Wrapper {
            #[serde(deserialize_with = "map_string_to_u64_from_string_or_number")]
            value: HashMap<String, u64>,
        }

        let parsed: Wrapper = serde_json::from_str(r#"{ "value": { "a": "3", "b": 4 } }"#).unwrap();
        assert_eq!(parsed.value.get("a"), Some(&3));
        assert_eq!(parsed.value.get("b"), Some(&4));
    }
}
