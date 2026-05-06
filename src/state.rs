use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::config;
use crate::session::SessionStore;

pub struct AppState {
    pub sessions: Arc<SessionStore>,
    /// Exposed-name → local directory mapping supplied at startup.
    pub folders: HashMap<String, PathBuf>,
}

impl AppState {
    pub fn new(folders: HashMap<String, PathBuf>) -> Self {
        Self {
            sessions: Arc::new(SessionStore::new()),
            folders,
        }
    }

    /// Resolve a `"name:///filename"` specifier to an absolute path.
    /// Delegates to `config::resolve_file`.
    pub fn resolve_file(&self, spec: &str) -> Result<PathBuf, String> {
        config::resolve_file(&self.folders, spec)
    }
}
