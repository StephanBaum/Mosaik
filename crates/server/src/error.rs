//! API errors: an HTTP status plus `{"error": "<code>", "detail": <object|null>}`
//! (M6 spec §6.1). Codes come from `mosaic_sync::wire::json::codes`.

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use mosaic_sync::wire::json::{self, ErrorBody, codes};
use serde::Serialize;

/// An error the API returns. It carries no secret and no user data.
#[derive(Debug)]
pub struct ApiError {
    /// HTTP status.
    pub status: StatusCode,
    /// The machine-readable code.
    pub code: &'static str,
    /// Structured detail, for the codes that define one.
    pub detail: Option<serde_json::Value>,
    /// `Retry-After` in seconds (rate limits).
    pub retry_after_s: Option<u32>,
}

/// The code of an unexpected server failure.
pub const INTERNAL: &str = "internal";

impl ApiError {
    /// An error with a status and a code.
    #[must_use]
    pub const fn new(status: StatusCode, code: &'static str) -> Self {
        Self {
            status,
            code,
            detail: None,
            retry_after_s: None,
        }
    }

    /// Adds structured detail.
    #[must_use]
    pub fn with_detail<T: Serialize>(mut self, detail: &T) -> Self {
        self.detail = serde_json::to_value(detail).ok();
        self
    }

    /// 400 `bad_request`.
    #[must_use]
    pub const fn bad_request() -> Self {
        Self::new(StatusCode::BAD_REQUEST, codes::BAD_REQUEST)
    }

    /// 401 `unauthorized`.
    #[must_use]
    pub const fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, codes::UNAUTHORIZED)
    }

    /// 403 `forbidden`.
    #[must_use]
    pub const fn forbidden() -> Self {
        Self::new(StatusCode::FORBIDDEN, codes::FORBIDDEN)
    }

    /// 404 `not_found`.
    #[must_use]
    pub const fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, codes::NOT_FOUND)
    }

    /// 413 `too_large`.
    #[must_use]
    pub const fn too_large() -> Self {
        Self::new(StatusCode::PAYLOAD_TOO_LARGE, codes::TOO_LARGE)
    }

    /// 429 `rate_limited` with a `Retry-After`.
    #[must_use]
    pub const fn rate_limited(retry_after_s: u32) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: codes::RATE_LIMITED,
            detail: None,
            retry_after_s: Some(retry_after_s),
        }
    }

    /// 409 with a code.
    #[must_use]
    pub const fn conflict(code: &'static str) -> Self {
        Self::new(StatusCode::CONFLICT, code)
    }

    /// 500. The cause is logged by the caller, never sent.
    #[must_use]
    pub const fn internal() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, INTERNAL)
    }

    /// The JSON body.
    #[must_use]
    pub fn body(&self) -> Vec<u8> {
        let body = ErrorBody {
            error: self.code.to_owned(),
            detail: self.detail.clone(),
        };
        json::to_vec(&body).unwrap_or_else(|_| b"{\"error\":\"internal\",\"detail\":null}".to_vec())
    }
}

impl From<rusqlite::Error> for ApiError {
    fn from(e: rusqlite::Error) -> Self {
        // SQLite messages name tables and columns, never row values.
        tracing::error!(error = %e, "database error");
        Self::internal()
    }
}

impl From<std::io::Error> for ApiError {
    fn from(e: std::io::Error) -> Self {
        tracing::error!(kind = ?e.kind(), "i/o error");
        Self::internal()
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (self.status, self.body()).into_response();
        let headers = response.headers_mut();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        if let Some(s) = self.retry_after_s
            && let Ok(v) = HeaderValue::from_str(&s.to_string())
        {
            headers.insert(header::RETRY_AFTER, v);
        }
        response
    }
}
