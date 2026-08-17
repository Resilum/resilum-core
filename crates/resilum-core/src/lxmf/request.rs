//! What can be wrong with a request handed in from outside.

use std::fmt;

/// Every way [`super::send::build_message`] and [`super::requeue::parse_request`]
/// can refuse their arguments. A caller matches on the variant; the strings
/// are for a human reading a log line.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RequestError {
    /// The request is not JSON, or not the shape this call expects.
    Json(String),
    /// A field that has to be lowercase hex is not, or decodes to the wrong
    /// number of bytes.
    Hex { field: &'static str },
    /// A field that has to be base64 is not.
    Base64 { field: &'static str },
    /// A delivery method outside `opportunistic` / `direct` / `propagated`.
    UnknownMethod(String),
    /// The fields parsed, but signing the message failed.
    Signing(String),
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(detail) => write!(f, "malformed request json: {detail}"),
            Self::Hex { field } => write!(f, "{field} is not the lowercase hex it must be"),
            Self::Base64 { field } => write!(f, "{field} is not valid base64"),
            Self::UnknownMethod(method) => write!(f, "unknown delivery method: {method}"),
            Self::Signing(detail) => write!(f, "could not build the message: {detail}"),
        }
    }
}

impl std::error::Error for RequestError {}
