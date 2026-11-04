//! Encryption key metadata and registration endpoints.

use crate::error::ServerError;
use crate::state::ServerState;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize)]
pub struct GetKeyPayload {
    #[serde(rename = "fileId")]
    pub file_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateKeyPayload {
    #[serde(rename = "fileId")]
    pub file_id: String,
    #[serde(rename = "keyId")]
    pub key_id: String,
    pub salt: String,
    #[serde(default = "default_iterations")]
    pub iterations: u32,
}

fn default_iterations() -> u32 {
    10000
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyInfoData {
    #[serde(rename = "keyId")]
    pub key_id: String,
    pub salt: String,
    pub iterations: u32,
}

/// Handler for `POST /user/get-key` (and `/user-get-key`).
pub async fn get_key_handler(
    State(state): State<ServerState>,
    Json(payload): Json<GetKeyPayload>,
) -> Result<Response, ServerError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    let row: Option<(Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT key_id, key_salt FROM files WHERE file_id = ?1",
            params![payload.file_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;

    let data = match row {
        Some((Some(kid), Some(salt))) => Some(KeyInfoData {
            key_id: kid,
            salt,
            iterations: 10000,
        }),
        _ => None,
    };

    Ok(Json(json!({
        "status": "ok",
        "data": data
    }))
    .into_response())
}

/// Handler for `POST /user/create-key` (and `/user-create-key`).
pub async fn create_key_handler(
    State(state): State<ServerState>,
    Json(payload): Json<CreateKeyPayload>,
) -> Result<Response, ServerError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    conn.execute(
        r#"
        UPDATE files
        SET key_id = ?1, key_salt = ?2
        WHERE file_id = ?3
        "#,
        params![payload.key_id, payload.salt, payload.file_id],
    )?;

    Ok(Json(json!({
        "status": "ok"
    }))
    .into_response())
}
