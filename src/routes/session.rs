use axum::{extract::State, http::HeaderMap, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{gringotts, routes::bearer_token, state::AppState};

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct OpenRequest {
    pub file: String,
    pub passphrase: String,
}

#[derive(Serialize)]
pub struct OpenResponse {
    pub token: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /api/session/open`
///
/// Body: `{ "file": "/path/to/file.grg", "passphrase": "secret" }`
///
/// Decrypts the gringotts file, stores the entries in memory and returns a
/// bearer token.  The session expires after 30 s of inactivity.
pub async fn open(
    State(state): State<Arc<AppState>>,
    Json(req): Json<OpenRequest>,
) -> Result<(StatusCode, Json<OpenResponse>), (StatusCode, String)> {
    let file = req.file.clone();
    let pwd = req.passphrase.clone();

    let entries = tokio::task::spawn_blocking(move || gringotts::load_file(&file, &pwd))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;

    let token = state.sessions.create(req.file, req.passphrase, entries);
    Ok((StatusCode::CREATED, Json(OpenResponse { token })))
}

/// `DELETE /api/session`
///
/// Saves the (possibly modified) entries back to the gringotts file and
/// destroys the session.
pub async fn close(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, String)> {
    let token = bearer_token(&headers)?;

    let session = state
        .sessions
        .remove(&token)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Invalid or expired session".to_owned()))?;

    let file = session.file_path.clone();
    let pwd = session.passphrase().to_owned();
    let entries = session.entries.clone();

    tokio::task::spawn_blocking(move || gringotts::save_file(&file, &pwd, &entries))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/session/keepalive`
///
/// Resets the 30-second inactivity timer.
pub async fn keepalive(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, String)> {
    let token = bearer_token(&headers)?;

    if state.sessions.touch(&token) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::UNAUTHORIZED, "Invalid or expired session".to_owned()))
    }
}
