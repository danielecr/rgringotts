# Source layout

```
src/
├── main.rs                  Entry point — CLI parsing, config loading, Tokio runtime,
│                            session expiry task, server bind
│
├── config.rs                Config struct (TOML) + resolve_file()
│                            • Deserialises rgringotts.toml: port, host, [folders]
│                            • merge() applies CLI overrides on top of the file
│                            • resolve_file(folders, "name:///file") → PathBuf
│                              (rejects path traversal; falls back to raw path when
│                               no folders are configured)
│
├── state.rs                 AppState — shared handle to SessionStore + folder map
│                            • AppState::resolve_file() delegates to config::resolve_file
│
├── session.rs               SessionStore + Session
│                            • Stores decrypted entries and passphrase (zeroized on drop)
│                            • Tracks last-activity; sessions expire after 30 s
│                            • Thread-safe via DashMap
│
├── gringotts/
│   ├── mod.rs               Public safe API over libgringotts
│   │                        • load_file(path, passphrase) → Vec<Entry>
│   │                        • save_file(path, passphrase, entries)
│   │                        • XML parser (quick-xml) for the <entry> format
│   │                          used by the gringotts application
│   └── ffi.rs               Raw `unsafe extern "C"` bindings to libgringotts
│                            • grg_context_initialize_defaults / grg_context_free
│                            • grg_update_gctx_from_file
│                            • grg_key_gen / grg_key_free
│                            • grg_decrypt_file / grg_encrypt_file / grg_free
│
└── routes/
    ├── mod.rs               Axum router (build_router) + bearer_token helper
    ├── folders.rs           Folder discovery handlers
    │                        • GET  /folders
    │                        • GET  /folders/{name}
    ├── session.rs           Session handlers
    │                        • POST   /api/session/open
    │                        • POST   /api/session/keepalive
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
  ├─► routes/folders.rs
  │       list_folders  → AppState.folders.keys()
  │       list_files    → tokio::fs::read_dir(folder)
  │
  ├─► routes/session.rs
  │       open      → AppState::resolve_file (config::resolve_file)
  │                 → gringotts::load_file (spawn_blocking)
  │                 → SessionStore::create
  │       keepalive → SessionStore::touch
  │       close     → SessionStore::remove
  │                 → gringotts::save_file (spawn_blocking)
  │
  └─► routes/entries.rs
          list / get / create / update / remove
              → SessionStore::with_session[_mut]
```

## Configuration precedence

```
[folders] in rgringotts.toml
        + --folder NAME=PATH  (CLI, merged, CLI wins on conflict)
        = AppState.folders  (HashMap<String, PathBuf>)
```

## Key dependencies

| Crate | Role |
|-------|------|
| `axum 0.8` | HTTP framework |
| `tokio` | Async runtime |
| `toml` | Config file deserialisation |
| `dashmap` | Lock-free concurrent hash map for the session store |
| `quick-xml` | Parse and serialise the gringotts XML entry format |
| `zeroize` | Wipe the passphrase from memory on session drop |
| `clap` | CLI argument parsing |
| `uuid` | Generate unguessable bearer tokens |
| `thiserror` | Typed error enums |
