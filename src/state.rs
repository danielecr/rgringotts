use std::sync::Arc;

use crate::session::SessionStore;

pub struct AppState {
    pub sessions: Arc<SessionStore>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(SessionStore::new()),
        }
    }
}
