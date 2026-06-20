//! Error types returned by this crate.

use std::fmt;

/// A convenient alias for results returned by this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Any error that can occur while talking to the Namecheap API.
///
/// Failures are split into distinct categories so callers can react to them
/// individually. In particular, errors reported by Namecheap *inside* the XML
/// response body are surfaced as [`Error::Api`] and kept separate from
/// transport-level HTTP failures ([`Error::Http`]).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The HTTP request could not be sent or the response could not be read
    /// (DNS failure, connection reset, timeout, TLS error, and so on).
    #[error("HTTP transport error: {0}")]
    Http(#[from] reqwest::Error),

    /// Namecheap accepted the request but returned one or more errors in the
    /// `<Errors>` block of the XML response.
    #[error(transparent)]
    Api(#[from] ApiError),

    /// The response body could not be decoded as the expected Namecheap XML.
    #[error("failed to decode Namecheap XML response: {0}")]
    Decode(#[from] quick_xml::DeError),

    /// The server returned a non-success HTTP status and the body was not a
    /// recognizable Namecheap error document.
    #[error("unexpected HTTP status {status}")]
    UnexpectedStatus {
        /// The HTTP status code returned by the server.
        status: reqwest::StatusCode,
        /// The raw response body, retained for debugging.
        body: String,
    },

    /// Namecheap reported success but the response contained no command result.
    #[error("the API reported success but returned no command result")]
    EmptyResponse,

    /// The [`Client`](crate::Client) was misconfigured (for example, a required
    /// credential was missing when calling
    /// [`ClientBuilder::build`](crate::ClientBuilder::build)).
    #[error("invalid client configuration: {0}")]
    Configuration(String),
}

/// One or more errors returned by Namecheap inside an XML response.
///
/// Namecheap signals failures with `Status="ERROR"` on the response envelope and
/// lists the individual problems in an `<Errors>` element. Each entry carries a
/// numeric code and a human-readable message; inspect [`ApiError::errors`] (or
/// use [`ApiError::has_code`]) to branch on specific codes.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ApiError {
    /// The individual errors reported by the API, in the order returned.
    pub errors: Vec<ApiErrorEntry>,
}

impl ApiError {
    /// Returns `true` if any returned error carries the given numeric code.
    ///
    /// Codes are compared as strings because Namecheap documents them that way
    /// (for example `"2030280"` for "TLD is not supported").
    #[must_use]
    pub fn has_code(&self, code: &str) -> bool {
        self.errors.iter().any(|entry| entry.number == code)
    }

    /// Returns the first reported error, if any.
    #[must_use]
    pub fn first(&self) -> Option<&ApiErrorEntry> {
        self.errors.first()
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Namecheap API error")?;
        for (index, entry) in self.errors.iter().enumerate() {
            let separator = if index == 0 { ": " } else { "; " };
            write!(f, "{separator}[{}] {}", entry.number, entry.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for ApiError {}

/// A single error reported by Namecheap.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ApiErrorEntry {
    /// The Namecheap error number (for example `"2019166"`).
    pub number: String,
    /// The human-readable error message.
    pub message: String,
}
