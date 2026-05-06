use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{gringotts::Entry, routes::bearer_token, state::AppState};

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

/// Summary returned by the list endpoint (omits the body for brevity).
#[derive(Serialize)]
pub struct EntrySummary {
    pub id: usize,
    pub title: String,
}

#[derive(Deserialize)]
pub struct EntryInput {
    pub title: String,
    pub body: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn unauthorized() -> (StatusCode, String) {
    (StatusCode::UNAUTHORIZED, "Invalid or expired session".to_owned())
}

fn not_found() -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, "Entry not found".to_owned())
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /api/entries` — list titles of all entries.
pub async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<EntrySummary>>, (StatusCode, String)> {
    let token = bearer_token(&headers)?;

    state
        .sessions
        .with_session(&token, |s| {
            s.entries
                .iter()
                .map(|e| EntrySummary { id: e.id, title: e.title.clone() })
                .collect::<Vec<_>>()
        })
        .map(Json)
        .ok_or_else(unauthorized)
}

/// `GET /api/entries/{id}` — get a single entry including its body.
pub async fn get_one(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<usize>,
) -> Result<Json<Entry>, (StatusCode, String)> {
    let token = bearer_token(&headers)?;

    let entry = state
        .sessions
        .with_session(&token, |s| s.entries.iter().find(|e| e.id == id).cloned())
        .ok_or_else(unauthorized)?
        .ok_or_else(not_found)?;

    Ok(Json(entry))
}

/// `POST /api/entries` — add a new entry.
pub async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<EntryInput>,
) -> Result<(StatusCode, Json<Entry>), (StatusCode, String)> {
    let token = bearer_token(&headers)?;

    let entry = state
        .sessions
        .with_session_mut(&token, |s| {
            let id = s.entries.iter().map(|e| e.id).max().map(|m| m + 1).unwrap_or(0);
            let e = Entry { id, title: input.title.clone(), body: input.body.clone() };
            s.entries.push(e.clone());
            e
        })
        .ok_or_else(unauthorized)?;

    Ok((StatusCode::CREATED, Json(entry)))
}

/// `PUT /api/entries/{id}` — update an existing entry.
pub async fn update(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<usize>,
    Json(input): Json<EntryInput>,
) -> Result<Json<Entry>, (StatusCode, String)> {
    let token = bearer_token(&headers)?;

    let result = state.sessions.with_session_mut(&token, |s| {
        s.entries.iter_mut().find(|e| e.id == id).map(|e| {
            e.title = input.title.clone();
            e.body = input.body.clone();
            e.clone()
        })
    });

    match result {
        Some(Some(e)) => Ok(Json(e)),
        Some(None) => Err(not_found()),
        None => Err(unauthorized()),
    }
}

/// `DELETE /api/entries/{id}` — remove an entry.
pub async fn remove(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<usize>,
) -> Result<StatusCode, (StatusCode, String)> {
    let token = bearer_token(&headers)?;

    let result = state.sessions.with_session_mut(&token, |s| {
        let before = s.entries.len();
        s.entries.retain(|e| e.id != id);
        s.entries.len() < before
    });

    match result {
        Some(true) => Ok(StatusCode::NO_CONTENT),
        Some(false) => Err(not_found()),
        None => Err(unauthorized()),
    }
}
