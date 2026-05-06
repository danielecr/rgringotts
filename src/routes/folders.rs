use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::state::AppState;

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
