//! Authentication, session management, and password validation matching Actual conventions.

use crate::error::ServerError;
use crate::state::ServerState;
use axum::async_trait;
use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Request payload for bootstrap and login endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPayload {
    pub password: String,
}

/// Extractor validating that the request carries a valid session token.
pub struct AuthSession {
    pub token: String,
}

#[async_trait]
impl FromRequestParts<ServerState> for AuthSession {
    type Rejection = ServerError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ServerState,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_token(&parts.headers).ok_or(ServerError::Unauthorized)?;

        // Verify token in database
        let is_valid = {
            let conn = state
                .db
                .lock()
                .map_err(|e| ServerError::Internal(e.to_string()))?;
            let now = Utc::now().timestamp();
            let count: i64 = conn.query_row(
                "SELECT COUNT(1) FROM sessions WHERE token = ?1 AND expires_at > ?2",
                params![token, now],
                |row| row.get(0),
            )?;
            count > 0
        };

        if !is_valid {
            return Err(ServerError::Unauthorized);
        }

        Ok(AuthSession { token })
    }
}

/// Extracts session token from headers (`X-Actual-Token`, `Authorization: Bearer <token>`, or Cookie).
fn extract_token(headers: &HeaderMap) -> Option<String> {
    if let Some(val) = headers.get("X-Actual-Token") {
        if let Ok(s) = val.to_str() {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    if let Some(val) = headers.get("Authorization") {
        if let Ok(s) = val.to_str() {
            if let Some(token) = s.strip_prefix("Bearer ") {
                let trimmed = token.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }

    // Check cookie
    if let Some(cookie_val) = headers.get("Cookie") {
        if let Ok(s) = cookie_val.to_str() {
            for part in s.split(';') {
                let part = part.trim();
                if let Some(token) = part.strip_prefix("actual-token=") {
                    return Some(token.trim().to_string());
                }
            }
        }
    }

    None
}

/// Handler for `GET /account/needs-bootstrap`.
pub async fn needs_bootstrap_handler(
    State(state): State<ServerState>,
) -> Result<Response, ServerError> {
    let bootstrapped = {
        let pass_lock = state
            .server_password
            .lock()
            .map_err(|e| ServerError::Internal(e.to_string()))?;
        pass_lock.is_some()
    };

    Ok(Json(json!({
        "status": "ok",
        "data": {
            "bootstrapped": bootstrapped
        }
    }))
    .into_response())
}

/// Handler for `POST /account/bootstrap`.
pub async fn bootstrap_handler(
    State(state): State<ServerState>,
    Json(payload): Json<PasswordPayload>,
) -> Result<Response, ServerError> {
    let mut pass_lock = state
        .server_password
        .lock()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    if pass_lock.is_some() {
        return Err(ServerError::Forbidden(
            "Server is already bootstrapped".into(),
        ));
    }

    *pass_lock = Some(payload.password);
    drop(pass_lock);

    // Create session token
    let token = create_session(&state)?;

    Ok(Json(json!({
        "status": "ok",
        "data": {
            "token": token
        }
    }))
    .into_response())
}

/// Handler for `POST /account/login`.
pub async fn login_handler(
    State(state): State<ServerState>,
    Json(payload): Json<PasswordPayload>,
) -> Result<Response, ServerError> {
    let pass_lock = state
        .server_password
        .lock()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    let expected = match pass_lock.as_ref() {
        Some(p) => p.clone(),
        None => return Err(ServerError::Forbidden("Server is not bootstrapped".into())),
    };
    drop(pass_lock);

    if payload.password != expected {
        return Err(ServerError::Unauthorized);
    }

    let token = create_session(&state)?;

    Ok(Json(json!({
        "status": "ok",
        "data": {
            "token": token
        }
    }))
    .into_response())
}

/// Handler for `POST /account/validate`.
pub async fn validate_handler(_session: AuthSession) -> Result<Response, ServerError> {
    Ok(Json(json!({
        "status": "ok"
    }))
    .into_response())
}

/// Helper to create and persist a new session token valid for 30 days.
pub fn create_session(state: &ServerState) -> Result<String, ServerError> {
    let token = Uuid::new_v4().to_string();
    let now = Utc::now();
    let created_at = now.to_rfc3339();
    let expires_at = now.timestamp() + (30 * 24 * 3600); // 30 days

    let conn = state
        .db
        .lock()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    conn.execute(
        "INSERT INTO sessions (token, user_id, created_at, expires_at) VALUES (?1, 'default', ?2, ?3)",
        params![token, created_at, expires_at],
    )?;

    Ok(token)
}
