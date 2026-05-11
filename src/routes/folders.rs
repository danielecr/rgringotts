use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateFileRequest {
    pub filename: String,
    pub passphrase: String,
}

/// `GET /folders` — list exposed folder names.
pub async fn list_folders(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    let mut names: Vec<String> = state.folders.keys().cloned().collect();
    names.sort();
    Json(names)
}

/// `GET /folders/{name}` — list files inside a mapped folder.
pub async fn list_files(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let folder = state
        .folders
        .get(&name)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Unknown folder '{name}'")))?
        .to_owned();

    let mut read_dir = tokio::fs::read_dir(&folder)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut files = Vec::new();
    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    {
        let ft = entry
            .file_type()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        if ft.is_file() {
            if let Some(n) = entry.file_name().to_str() {
                files.push(n.to_owned());
            }
        }
    }

    files.sort();
    Ok(Json(files))
}

/// `POST /folders/{name}` — create a new empty vault file inside a mapped folder.
///
/// Body: `{ "filename": "myfile", "passphrase": "secret" }`
///
/// The filename must be a plain name (no path separators).
pub async fn create_file(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<CreateFileRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let filename = req.filename.trim().to_owned();
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains(std::path::MAIN_SEPARATOR)
        || filename == "."
        || filename == ".."
    {
        return Err((StatusCode::BAD_REQUEST, "Invalid filename".to_owned()));
    }

    let folder = state
        .folders
        .get(&name)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Unknown folder '{name}'")))?
        .to_owned();

    let file_path = folder.join(&filename);
    if file_path.exists() {
        return Err((StatusCode::CONFLICT, format!("File '{filename}' already exists")));
    }

    let path_str = file_path.to_string_lossy().into_owned();
    let pwd = req.passphrase.clone();
    tokio::task::spawn_blocking(move || crate::gringotts::save_file(&path_str, &pwd, &[]))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}
