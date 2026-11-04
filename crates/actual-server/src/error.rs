//! Error types and HTTP response formatting for actual-server.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

/// Server error types across database, authentication, and sync operations.
#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protobuf decode error: {0}")]
    ProtobufDecode(#[from] prost::DecodeError),

    #[error("Protobuf encode error: {0}")]
    ProtobufEncode(#[from] prost::EncodeError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Authentication required")]
    Unauthorized,

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            Self::Forbidden(_) => (StatusCode::FORBIDDEN, self.to_string()),
            Self::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            Self::Database(_)
            | Self::Io(_)
            | Self::ProtobufDecode(_)
            | Self::ProtobufEncode(_)
            | Self::Json(_)
            | Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = Json(json!({
            "status": "error",
            "reason": msg,
        }));

        (status, body).into_response()
    }
}
