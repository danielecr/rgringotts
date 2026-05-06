# Source layout

```
src/
├── main.rs                  Entry point — CLI parsing, Tokio runtime, session expiry task, server bind
│
├── state.rs                 AppState — shared handle to the SessionStore
│
├── session.rs               SessionStore + Session
│                            • Stores decrypted entries and the passphrase (zeroized on drop)
│                            • Tracks last-activity timestamp; sessions expire after 30 s of inactivity
│                            • Thread-safe via DashMap — no Mutex required on the hot path
│
├── gringotts/
│   ├── mod.rs               Public safe API over libgringotts
│   │                        • load_file(path, passphrase) → Vec<Entry>
│   │                        • save_file(path, passphrase, entries)
│   │                        • XML parser (quick-xml) that reads/writes the <entry> format
│   │                          used by the gringotts application
│   └── ffi.rs               Raw `unsafe extern "C"` bindings to libgringotts
│                            • grg_context_initialize_defaults / grg_context_free
│                            • grg_update_gctx_from_file
│                            • grg_key_gen / grg_key_free
│                            • grg_decrypt_file / grg_encrypt_file
│                            • grg_free
│
└── routes/
    ├── mod.rs               Axum router (build_router) + bearer_token helper
    ├── session.rs           Session handlers
    │                        • POST  /api/session/open
    │                        • POST  /api/session/keepalive
    │                        • DELETE /api/session
    └── entries.rs           Entry CRUD handlers
                             • GET    /api/entries
                             • POST   /api/entries
                             • GET    /api/entries/{id}
                             • PUT    /api/entries/{id}
                             • DELETE /api/entries/{id}
```

## Data flow

```
Client
  │
  │  HTTP (plain — tunnel via SSH)
  ▼
Axum router  (routes/mod.rs)
  │
  ├─► routes/session.rs
  │       open      → gringotts::load_file (spawn_blocking) → SessionStore::create
  │       keepalive → SessionStore::touch
  │       close     → SessionStore::remove → gringotts::save_file (spawn_blocking)
  │
  └─► routes/entries.rs
          list / get / create / update / remove
              → SessionStore::with_session[_mut]
```

## Key dependencies

| Crate | Role |
|-------|------|
| `axum 0.8` | HTTP framework |
| `tokio` | Async runtime |
| `dashmap` | Lock-free concurrent hash map for the session store |
| `quick-xml` | Parse and serialise the gringotts XML entry format |
| `zeroize` | Wipe the passphrase from memory on session drop |
| `clap` | CLI argument parsing (`-p PORT`, `-h HOST`) |
| `uuid` | Generate unguessable bearer tokens |
| `thiserror` | Typed error enums |
