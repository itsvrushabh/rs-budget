//! User budget archive backup, restore, and file metadata endpoints.

use crate::error::ServerError;
use crate::state::ServerState;
use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct FileIdQuery {
    #[serde(rename = "fileId")]
    pub file_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFilenamePayload {
    #[serde(rename = "fileId")]
    pub file_id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct FileIdPayload {
    #[serde(rename = "fileId")]
    pub file_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserFileItem {
    #[serde(rename = "fileId")]
    pub file_id: String,
    #[serde(rename = "groupId")]
    pub group_id: String,
    pub name: String,
    #[serde(rename = "encryptMeta")]
    pub encrypt_meta: Option<serde_json::Value>,
    pub deleted: bool,
}

/// Handler for `POST /sync/upload-user-file`.
pub async fn upload_user_file_handler(
    State(state): State<ServerState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ServerError> {
    let file_id = headers
        .get("X-Actual-File-Id")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ServerError::BadRequest("Missing X-Actual-File-Id header".into()))?
        .to_string();

    let name = headers
        .get("X-Actual-Name")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("Budget")
        .to_string();

    let group_id = headers
        .get("X-Actual-Group-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let encrypt_meta = headers
        .get("X-Actual-Encrypt-Meta")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // 1. Write file to disk
    let file_path = state
        .data_dir
        .join("user-files")
        .join(format!("{file_id}.actual"));
    fs::write(&file_path, &body)?;

    // 2. Update files table
    let now = Utc::now().to_rfc3339();
    {
        let conn = state
            .db
            .lock()
            .map_err(|e| ServerError::Internal(e.to_string()))?;
        conn.execute(
            r#"
            INSERT INTO files (file_id, group_id, name, encrypt_meta, deleted, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, 0, ?5, ?5)
            ON CONFLICT(file_id) DO UPDATE SET
                group_id = ?2,
                name = ?3,
                encrypt_meta = ?4,
                deleted = 0,
                updated_at = ?5
            "#,
            params![file_id, group_id, name, encrypt_meta, now],
        )?;
    }

    Ok(Json(json!({
        "status": "ok",
        "data": {
            "fileId": file_id,
            "groupId": group_id
        }
    }))
    .into_response())
}

/// Handler for `GET /sync/download-user-file`.
pub async fn download_user_file_handler(
    State(state): State<ServerState>,
    Query(query): Query<FileIdQuery>,
) -> Result<Response, ServerError> {
    let file_path = state
        .data_dir
        .join("user-files")
        .join(format!("{}.actual", query.file_id));
    if !file_path.exists() {
        return Err(ServerError::NotFound(format!(
            "File '{}' not found",
            query.file_id
        )));
    }

    let bytes = fs::read(&file_path)?;

    let mut response = bytes.into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, "application/octet-stream".parse().unwrap());

    Ok(response)
}

/// Handler for `GET /sync/list-user-files`.
pub async fn list_user_files_handler(
    State(state): State<ServerState>,
) -> Result<Response, ServerError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    let mut stmt = conn.prepare(
        "SELECT file_id, group_id, name, encrypt_meta, deleted FROM files WHERE deleted = 0 ORDER BY updated_at DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        let file_id: String = row.get(0)?;
        let group_id: String = row.get(1)?;
        let name: String = row.get(2)?;
        let enc_str: Option<String> = row.get(3)?;
        let deleted: i64 = row.get(4)?;

        let encrypt_meta = enc_str.and_then(|s| serde_json::from_str(&s).ok());

        Ok(UserFileItem {
            file_id,
            group_id,
            name,
            encrypt_meta,
            deleted: deleted != 0,
        })
    })?;

    let mut files = Vec::new();
    for r in rows {
        files.push(r?);
    }

    Ok(Json(json!({
        "status": "ok",
        "data": files
    }))
    .into_response())
}

/// Handler for `GET /sync/get-user-file-info`.
pub async fn get_user_file_info_handler(
    State(state): State<ServerState>,
    Query(query): Query<FileIdQuery>,
) -> Result<Response, ServerError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    let file_info: Option<UserFileItem> = conn
        .query_row(
            "SELECT file_id, group_id, name, encrypt_meta, deleted FROM files WHERE file_id = ?1",
            params![query.file_id],
            |row| {
                let file_id: String = row.get(0)?;
                let group_id: String = row.get(1)?;
                let name: String = row.get(2)?;
                let enc_str: Option<String> = row.get(3)?;
                let deleted: i64 = row.get(4)?;

                let encrypt_meta = enc_str.and_then(|s| serde_json::from_str(&s).ok());

                Ok(UserFileItem {
                    file_id,
                    group_id,
                    name,
                    encrypt_meta,
                    deleted: deleted != 0,
                })
            },
        )
        .optional()?;

    let item = file_info
        .ok_or_else(|| ServerError::NotFound(format!("File '{}' not found", query.file_id)))?;

    Ok(Json(json!({
        "status": "ok",
        "data": item
    }))
    .into_response())
}

/// Handler for `POST /sync/update-user-filename`.
pub async fn update_user_filename_handler(
    State(state): State<ServerState>,
    Json(payload): Json<UpdateFilenamePayload>,
) -> Result<Response, ServerError> {
    let now = Utc::now().to_rfc3339();
    let conn = state
        .db
        .lock()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    let rows_affected = conn.execute(
        "UPDATE files SET name = ?1, updated_at = ?2 WHERE file_id = ?3",
        params![payload.name, now, payload.file_id],
    )?;

    if rows_affected == 0 {
        return Err(ServerError::NotFound(format!(
            "File '{}' not found",
            payload.file_id
        )));
    }

    Ok(Json(json!({
        "status": "ok"
    }))
    .into_response())
}

/// Handler for `POST /sync/delete-user-file`.
pub async fn delete_user_file_handler(
    State(state): State<ServerState>,
    Json(payload): Json<FileIdPayload>,
) -> Result<Response, ServerError> {
    let now = Utc::now().to_rfc3339();
    let conn = state
        .db
        .lock()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    conn.execute(
        "UPDATE files SET deleted = 1, updated_at = ?1 WHERE file_id = ?2",
        params![now, payload.file_id],
    )?;

    Ok(Json(json!({
        "status": "ok"
    }))
    .into_response())
}

/// Handler for `POST /sync/reset-user-file`.
pub async fn reset_user_file_handler(
    State(state): State<ServerState>,
    Json(payload): Json<FileIdPayload>,
) -> Result<Response, ServerError> {
    let new_group_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let conn = state
        .db
        .lock()
        .map_err(|e| ServerError::Internal(e.to_string()))?;
    conn.execute(
        "UPDATE files SET group_id = ?1, updated_at = ?2 WHERE file_id = ?3",
        params![new_group_id, now, payload.file_id],
    )?;

    // Clear messages for old group
    conn.execute(
        "DELETE FROM messages WHERE file_id = ?1",
        params![payload.file_id],
    )?;

    conn.execute(
        "DELETE FROM merkle_tries WHERE file_id = ?1",
        params![payload.file_id],
    )?;

    Ok(Json(json!({
        "status": "ok",
        "data": {
            "groupId": new_group_id
        }
    }))
    .into_response())
}
