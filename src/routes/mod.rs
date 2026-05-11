use axum::{
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Router,
};

use std::sync::Arc;

use crate::state::AppState;

mod entries;
mod folders;
mod session;

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Folder discovery
        .route("/folders", get(folders::list_folders))
        .route("/folders/{name}", get(folders::list_files).post(folders::create_file))
        // Session lifecycle
        .route("/api/session/open", post(session::open))
        .route("/api/session", delete(session::close))
        .route("/api/session/keepalive", post(session::keepalive))
        // Entries CRUD
        .route("/api/entries", get(entries::list))
        .route("/api/entries", post(entries::create))
        .route("/api/entries/{id}", get(entries::get_one))
        .route("/api/entries/{id}", put(entries::update))
        .route("/api/entries/{id}", delete(entries::remove))
        .with_state(state)
}

/// Extract the bearer token from the `Authorization` header.
pub(crate) fn bearer_token(headers: &HeaderMap) -> Result<String, (StatusCode, String)> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_owned())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Missing or invalid Authorization header".to_owned(),
            )
        })
}
