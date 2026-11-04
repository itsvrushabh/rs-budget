//! Axum application construction, middleware configuration, and router assembly.

use crate::auth::{bootstrap_handler, login_handler, needs_bootstrap_handler, validate_handler};
use crate::routes::keys::{create_key_handler, get_key_handler};
use crate::routes::sync::sync_handler;
use crate::routes::user_files::{
    delete_user_file_handler, download_user_file_handler, get_user_file_info_handler,
    list_user_files_handler, reset_user_file_handler, update_user_filename_handler,
    upload_user_file_handler,
};
use crate::state::ServerState;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use std::path::PathBuf;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

/// Builds the complete Axum HTTP router with all sync, authentication, and archive endpoints.
pub fn create_app(state: ServerState) -> Router {
    create_app_with_options(state, None)
}

/// Builds the complete Axum HTTP router with optional static web client directory serving.
pub fn create_app_with_options(state: ServerState, web_dir: Option<PathBuf>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let router = Router::new()
        // Account & Authentication
        .route("/account/needs-bootstrap", get(needs_bootstrap_handler))
        .route("/account/bootstrap", post(bootstrap_handler))
        .route("/account/login", post(login_handler))
        .route("/account/validate", post(validate_handler))
        // CRDT Sync
        .route("/sync/sync", post(sync_handler))
        .route("/sync", post(sync_handler))
        // User Files / Backups
        .route("/sync/upload-user-file", post(upload_user_file_handler))
        .route("/sync/download-user-file", get(download_user_file_handler))
        .route("/sync/list-user-files", get(list_user_files_handler))
        .route("/sync/get-user-file-info", get(get_user_file_info_handler))
        .route(
            "/sync/update-user-filename",
            post(update_user_filename_handler),
        )
        .route("/sync/delete-user-file", post(delete_user_file_handler))
        .route("/sync/reset-user-file", post(reset_user_file_handler))
        // Key Management
        .route("/user/get-key", post(get_key_handler))
        .route("/user-get-key", post(get_key_handler))
        .route("/user/create-key", post(create_key_handler))
        .route("/user-create-key", post(create_key_handler))
        // Allow up to 50MB for encrypted budget backups
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
        .layer(cors)
        .with_state(state);

    if let Some(dir) = web_dir {
        if dir.exists() {
            let index_file = dir.join("index.html");
            if index_file.exists() {
                return router
                    .fallback_service(ServeDir::new(&dir).fallback(ServeFile::new(index_file)));
            } else {
                return router.fallback_service(ServeDir::new(&dir));
            }
        }
    }

    router
}
