//! Internal helpers for decoding the Namecheap XML response envelope.
//!
//! Every Namecheap response is wrapped in an `<ApiResponse>` element that
//! carries a `Status` attribute (`OK` or `ERROR`), an `<Errors>` block, and a
//! `<CommandResponse>` block holding the command-specific payload. We decode the
//! body twice: once to read the status and any errors (regardless of payload
//! shape), and, when the call succeeded, a second time to extract the typed
//! payload. Splitting the two passes keeps error handling robust even when the
//! `<CommandResponse>` is empty on failure.

use serde::de::DeserializeOwned;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer};

use crate::error::{ApiError, ApiErrorEntry, Error, Result};

/// Decodes a raw response body into the typed command payload `T`.
///
/// A body is treated as an authoritative Namecheap response only when it parses
/// into the envelope *and* carries a `Status` attribute. Otherwise it is handled
/// as a transport- or format-level failure, so that an HTML error page returned
/// with a 5xx status surfaces as [`Error::UnexpectedStatus`] rather than as a
/// confusingly empty API error.
pub(crate) fn parse<T>(status: reqwest::StatusCode, body: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    match quick_xml::de::from_str::<Envelope>(body) {
        Ok(envelope) if !envelope.status.trim().is_empty() => {
            if envelope.is_ok() {
                let payload: CommandEnvelope<T> = quick_xml::de::from_str(body)?;
                payload.command_response.ok_or(Error::EmptyResponse)
            } else {
                Err(Error::Api(envelope.into_api_error()))
            }
        }
        Ok(_) => fallback(status, body, None),
        Err(decode_error) => fallback(status, body, Some(decode_error)),
    }
}

/// Builds the error for a body that is not a recognizable Namecheap envelope.
fn fallback<T>(
    status: reqwest::StatusCode,
    body: &str,
    decode_error: Option<quick_xml::DeError>,
) -> Result<T> {
    if !status.is_success() {
        return Err(Error::UnexpectedStatus {
            status,
            body: body.to_owned(),
        });
    }
    Err(Error::Decode(decode_error.unwrap_or_else(|| {
        quick_xml::DeError::custom("response was not a Namecheap ApiResponse document")
    })))
}

/// Lightweight view of the envelope used to read status and errors only.
#[derive(Debug, Deserialize)]
#[serde(rename = "ApiResponse")]
struct Envelope {
    #[serde(rename = "@Status", default)]
    status: String,
    #[serde(rename = "Errors", default)]
    errors: RawErrors,
}

impl Envelope {
    fn is_ok(&self) -> bool {
        self.status.eq_ignore_ascii_case("OK")
    }

    fn into_api_error(self) -> ApiError {
        ApiError {
            errors: self
                .errors
                .errors
                .into_iter()
                .map(|raw| ApiErrorEntry {
                    number: raw.number,
                    message: raw.message.trim().to_owned(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct RawErrors {
    #[serde(rename = "Error", default)]
    errors: Vec<RawError>,
}

#[derive(Debug, Deserialize)]
struct RawError {
    #[serde(rename = "@Number", default)]
    number: String,
    #[serde(rename = "$text", default)]
    message: String,
}

/// View of the envelope used to read the typed command payload only.
#[derive(Debug, Deserialize)]
#[serde(rename = "ApiResponse")]
struct CommandEnvelope<T> {
    #[serde(rename = "CommandResponse")]
    command_response: Option<T>,
}

/// Parses the boolean spellings Namecheap uses in attribute values.
fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" | "enabled" => Some(true),
        "false" | "no" | "0" | "disabled" => Some(false),
        _ => None,
    }
}

/// Deserializes a required boolean attribute, tolerating Namecheap's spellings.
pub(crate) fn de_bool<'de, D>(deserializer: D) -> std::result::Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    parse_bool(&raw)
        .ok_or_else(|| serde::de::Error::custom(format!("invalid boolean value: {raw:?}")))
}

/// Deserializes an optional boolean attribute, treating empty values as `None`.
pub(crate) fn de_opt_bool<'de, D>(deserializer: D) -> std::result::Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<String>::deserialize(deserializer)? {
        Some(raw) if !raw.trim().is_empty() => parse_bool(&raw)
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid boolean value: {raw:?}"))),
        _ => Ok(None),
    }
}

/// Deserializes an optional value that parses from a string, treating empty
/// attribute values as `None` rather than as a parse failure.
pub(crate) fn de_opt_from_str<'de, D, T>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match Option::<String>::deserialize(deserializer)? {
        Some(raw) if !raw.trim().is_empty() => raw
            .trim()
            .parse::<T>()
            .map(Some)
            .map_err(serde::de::Error::custom),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use reqwest::StatusCode;

    #[derive(Debug, Deserialize)]
    struct Empty {}

    #[test]
    fn parse_bool_accepts_namecheap_spellings() {
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("FALSE"), Some(false));
        assert_eq!(parse_bool(" Yes "), Some(true));
        assert_eq!(parse_bool("0"), Some(false));
        assert_eq!(parse_bool("enabled"), Some(true));
        assert_eq!(parse_bool("maybe"), None);
    }

    #[test]
    fn error_envelope_becomes_api_error() {
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
        <ApiResponse Status="ERROR" xmlns="http://api.namecheap.com/xml.response">
          <Errors>
            <Error Number="2019166">Domain not found</Error>
            <Error Number="3031510">Insufficient funds</Error>
          </Errors>
          <CommandResponse />
        </ApiResponse>"#;

        let result = parse::<Empty>(StatusCode::OK, body);
        match result {
            Err(Error::Api(api)) => {
                assert!(api.has_code("2019166"));
                assert!(api.has_code("3031510"));
                assert_eq!(api.first().unwrap().message, "Domain not found");
                assert_eq!(api.errors.len(), 2);
            }
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    #[test]
    fn non_success_status_with_garbage_body_is_unexpected_status() {
        let result = parse::<Empty>(StatusCode::INTERNAL_SERVER_ERROR, "<html>503</html>");
        assert!(matches!(result, Err(Error::UnexpectedStatus { .. })));
    }

    #[test]
    fn ok_without_command_response_is_empty_response() {
        let body = r#"<ApiResponse Status="OK" xmlns="http://api.namecheap.com/xml.response">
          <Errors />
        </ApiResponse>"#;
        let result = parse::<Empty>(StatusCode::OK, body);
        assert!(matches!(result, Err(Error::EmptyResponse)));
    }
}
