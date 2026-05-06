use std::time::{Duration, Instant};

use dashmap::DashMap;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::gringotts::Entry;

const SESSION_TIMEOUT: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

pub struct Session {
    pub file_path: String,
    pub entries: Vec<Entry>,
    /// Passphrase kept for re-encryption on explicit close.
    /// Wrapped in `Zeroizing` so it is overwritten on drop.
    passphrase: Zeroizing<String>,
    pub last_activity: Instant,
}

impl Session {
    fn new(file_path: String, passphrase: String, entries: Vec<Entry>) -> Self {
        Self {
            file_path,
            entries,
            passphrase: Zeroizing::new(passphrase),
            last_activity: Instant::now(),
        }
    }

    pub fn passphrase(&self) -> &str {
        &self.passphrase
    }

    fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn is_expired(&self) -> bool {
        self.last_activity.elapsed() > SESSION_TIMEOUT
    }
}

// ---------------------------------------------------------------------------
// SessionStore
// ---------------------------------------------------------------------------

pub struct SessionStore {
    map: DashMap<String, Session>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self { map: DashMap::new() }
    }

    /// Create a new session and return its bearer token.
    pub fn create(&self, file_path: String, passphrase: String, entries: Vec<Entry>) -> String {
        let token = Uuid::new_v4().to_string();
        self.map.insert(token.clone(), Session::new(file_path, passphrase, entries));
        token
    }

    /// Remove a session and return it so the caller can save the file.
    pub fn remove(&self, token: &str) -> Option<Session> {
        self.map.remove(token).map(|(_, v)| v)
    }

    /// Reset the inactivity timer. Returns `false` if the session is gone / expired.
    pub fn touch(&self, token: &str) -> bool {
        if let Some(mut s) = self.map.get_mut(token) {
            if s.is_expired() {
                drop(s);
                self.map.remove(token);
                return false;
            }
            s.touch();
            true
        } else {
            false
        }
    }

    /// Execute `f` with a shared reference to the session, also touching it.
    ///
    /// Returns `None` if the session does not exist or has expired.
    pub fn with_session<F, R>(&self, token: &str, f: F) -> Option<R>
    where
        F: FnOnce(&Session) -> R,
    {
        let mut s = self.map.get_mut(token)?;
        if s.is_expired() {
            drop(s);
            self.map.remove(token);
            return None;
        }
        s.touch();
        Some(f(&*s))
    }

    /// Execute `f` with an exclusive reference to the session, also touching it.
    ///
    /// Returns `None` if the session does not exist or has expired.
    pub fn with_session_mut<F, R>(&self, token: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut Session) -> R,
    {
        let mut s = self.map.get_mut(token)?;
        if s.is_expired() {
            drop(s);
            self.map.remove(token);
            return None;
        }
        s.touch();
        Some(f(&mut *s))
    }

    /// Drop all sessions that have exceeded the inactivity timeout.
    pub fn expire_old(&self) {
        self.map.retain(|_, s| !s.is_expired());
    }
}
