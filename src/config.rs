use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Config struct
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    /// TCP port to listen on.
    pub port: Option<u16>,
    /// Host / IP address to bind.
    pub host: Option<String>,
    /// Exposed-name → local directory path.
    #[serde(default)]
    pub folders: HashMap<String, PathBuf>,
}

impl Config {
    /// Load a config from a TOML file.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let txt = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
        toml::from_str(&txt).map_err(|e| format!("TOML error in {}: {e}", path.display()))
    }

    /// Merge `other` on top of `self` — other wins for scalar fields and
    /// folder names that appear in both.
    pub fn merge(&mut self, other: Self) {
        if other.port.is_some() {
            self.port = other.port;
        }
        if other.host.is_some() {
            self.host = other.host;
        }
        self.folders.extend(other.folders);
    }
}

// ---------------------------------------------------------------------------
// File-spec resolution
// ---------------------------------------------------------------------------

/// Resolve a `"name:///filename"` specifier to an absolute `PathBuf`.
///
/// * `name` is looked up in `folders`.
/// * The filename part must be a plain name (no `..`, no leading `/`).
///
/// If `folders` is empty and `spec` contains no `://`, the spec is returned
/// as-is (absolute-path fallback for setups without folder restrictions).
pub fn resolve_file(folders: &HashMap<String, PathBuf>, spec: &str) -> Result<PathBuf, String> {
    if let Some(sep) = spec.find("://") {
        let name = &spec[..sep];
        // Skip all leading slashes after "://"
        let rel = spec[sep + 3..].trim_start_matches('/');

        if rel.is_empty() {
            return Err("No filename specified after the folder name".to_owned());
        }
        if !is_safe_filename(rel) {
            return Err("Path traversal is not allowed".to_owned());
        }

        let folder = folders
            .get(name)
            .ok_or_else(|| format!("Unknown folder '{name}'"))?;

        Ok(folder.join(rel))
    } else if folders.is_empty() {
        // Legacy / no-restriction mode: accept a raw path.
        Ok(PathBuf::from(spec))
    } else {
        Err("File must be specified as 'folder_name:///filename' when folders are configured"
            .to_owned())
    }
}

/// Returns `true` only when every path component is a normal file/dir name
/// (no `..`, no root `/`, no Windows drive letters).
fn is_safe_filename(rel: &str) -> bool {
    use std::path::Component;
    std::path::Path::new(rel)
        .components()
        .all(|c| matches!(c, Component::Normal(_)))
}
