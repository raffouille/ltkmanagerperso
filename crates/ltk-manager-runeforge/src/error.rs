//! Why a call to RuneForge failed, as the remedy that the app offers for it.

use ltk_manager_base::error::{AppError, DomainError, ErrorKind};
use reqwest::StatusCode;
use reqwest::header::{HeaderMap, RETRY_AFTER};
use serde::{Deserialize, Serialize};

use crate::wire::{Finding, PublishChecklist, Scope};

/// Why a call to RuneForge failed.
///
/// Sent over IPC whole, as the `error` of the `RUNEFORGE` code, so the frontend switches on
/// `kind` and renders the data that a variant carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(
    tag = "kind",
    rename_all = "SCREAMING_SNAKE_CASE",
    rename_all_fields = "camelCase"
)]
pub enum RuneforgeError {
    /// The app has no stored token.
    #[error("no RuneForge account is connected")]
    NotConnected,

    /// The token is malformed, unknown, revoked or expired.
    #[error("RuneForge refused the token")]
    Unauthorized,

    /// The token lacks `scope`, or the account may not do this if `scope` is `None`.
    #[error("RuneForge refused the request for the token")]
    Forbidden { scope: Option<Scope> },

    /// The post or the upload does not exist for the account.
    #[error("RuneForge has no such post or upload")]
    NotFound,

    /// The server refused the package for `findings`.
    #[error("RuneForge refused the package")]
    PackageRejected { findings: Vec<Finding> },

    /// The post already has the tag. `suggested_tag` is the next free patch.
    #[error("the post already has the tag")]
    TagTaken { suggested_tag: String },

    /// The upload is past the state that the call needs.
    #[error("the upload is past the state the call needs")]
    UploadState,

    /// The post did not publish. `checklist` lists what it lacks.
    #[error("RuneForge did not publish the post")]
    PublishRefused { checklist: PublishChecklist },

    /// An earlier request with the same `Idempotency-Key` differs or still runs, or the post is
    /// under moderation.
    #[error("the request conflicts with an earlier one")]
    Conflict,

    /// The package is over the 1 GiB limit.
    #[error("the package is over the 1 GiB limit")]
    TooLarge,

    /// The server refused the request because the token sent over 120 requests in a minute.
    /// `retry_after` is in seconds.
    #[error("too many RuneForge requests")]
    RateLimited { retry_after: Option<u64> },

    /// The storage host refused the PUT of the package bytes with `status`.
    #[error("the package upload failed with status {status}")]
    Storage { status: u16 },

    /// The server answered with `status` and an error `code` that has no variant of its own.
    #[error("RuneForge answered with status {status} ({code})")]
    Http { status: u16, code: String },

    /// The answer is not in the shape that the route documents.
    #[error("RuneForge's answer is not readable: {detail}")]
    Malformed { detail: String },

    /// The server was not reached, or the connection broke.
    #[error("RuneForge is unreachable: {detail}")]
    Offline { detail: String },

    /// The user cancelled the publish.
    #[error("the publish was cancelled")]
    Cancelled,

    /// The OS credential store failed.
    #[error("the credential store failed: {detail}")]
    Credential { detail: String },
}

impl From<RuneforgeError> for AppError {
    fn from(error: RuneforgeError) -> Self {
        Self::Domain(DomainError::new(ErrorKind::Runeforge, error))
    }
}

/// The `{ error }` body of every failed `/api/v1` call.
#[derive(Debug, Deserialize)]
struct ErrorBody {
    error: ErrorPayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorPayload {
    code: String,
    #[serde(default)]
    findings: Option<Vec<Finding>>,
    #[serde(default)]
    suggested_tag: Option<String>,
    #[serde(default)]
    checklist: Option<PublishChecklist>,
    #[serde(default)]
    scope: Option<Scope>,
}

impl RuneforgeError {
    /// Reads a failed API answer of `status`, with its `headers` and `body`, into a variant.
    ///
    /// Maps the `code` of the body and never parses its `message`. A body that is not the error
    /// shape maps by the status alone.
    pub(crate) fn from_answer(status: StatusCode, headers: &HeaderMap, body: &[u8]) -> Self {
        let Ok(ErrorBody { error }) = serde_json::from_slice::<ErrorBody>(body) else {
            return Self::from_status(status, headers);
        };

        match error.code.as_str() {
            "unauthorized" => Self::Unauthorized,
            "forbidden" => Self::Forbidden { scope: error.scope },
            "not_found" => Self::NotFound,
            "package_rejected" => Self::PackageRejected {
                findings: error.findings.unwrap_or_default(),
            },
            "tag_taken" => match error.suggested_tag {
                Some(suggested_tag) => Self::TagTaken { suggested_tag },
                None => Self::http(status, error.code),
            },
            "upload_state" => Self::UploadState,
            "publish_refused" => match error.checklist {
                Some(checklist) => Self::PublishRefused { checklist },
                None => Self::http(status, error.code),
            },
            "conflict" => Self::Conflict,
            "payload_too_large" => Self::TooLarge,
            "too_many_requests" => Self::RateLimited {
                retry_after: retry_after(headers),
            },
            _ => Self::http(status, error.code),
        }
    }

    fn from_status(status: StatusCode, headers: &HeaderMap) -> Self {
        match status {
            StatusCode::UNAUTHORIZED => Self::Unauthorized,
            StatusCode::FORBIDDEN => Self::Forbidden { scope: None },
            StatusCode::NOT_FOUND => Self::NotFound,
            StatusCode::PAYLOAD_TOO_LARGE => Self::TooLarge,
            StatusCode::TOO_MANY_REQUESTS => Self::RateLimited {
                retry_after: retry_after(headers),
            },
            _ => Self::http(status, "unknown".to_owned()),
        }
    }

    fn http(status: StatusCode, code: String) -> Self {
        Self::Http {
            status: status.as_u16(),
            code,
        }
    }

    /// Reads a transport failure of `reqwest` into a variant.
    pub(crate) fn transport(error: &reqwest::Error) -> Self {
        if error.is_decode() {
            return Self::Malformed {
                detail: error.to_string(),
            };
        }

        /* The URL is the API host or a presigned storage URL. Neither carries the token, but a
        presigned URL carries a signature, so the detail drops it. */
        Self::Offline {
            detail: without_url(error),
        }
    }

    /// Returns `true` if the token was refused, which marks the connection invalid.
    #[must_use]
    pub fn is_unauthorized(&self) -> bool {
        matches!(self, Self::Unauthorized)
    }
}

/// Returns the wait of a `Retry-After` header in seconds. The API sends seconds, never a date.
fn retry_after(headers: &HeaderMap) -> Option<u64> {
    headers.get(RETRY_AFTER)?.to_str().ok()?.trim().parse().ok()
}

/// Returns the text of `error` and its sources, without the URL that `error` names.
fn without_url(error: &reqwest::Error) -> String {
    let mut text = if error.is_timeout() {
        "the request timed out".to_owned()
    } else if error.is_connect() {
        "the connection failed".to_owned()
    } else {
        "the request failed".to_owned()
    };

    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }

    text
}
