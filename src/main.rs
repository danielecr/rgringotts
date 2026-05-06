mod config;
mod gringotts;
mod routes;
mod session;
mod state;

use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};

use clap::Parser;
use tokio::time::Duration;
use tracing_subscriber::EnvFilter;

use state::AppState;

/// Remote gateway to a gringotts encrypted file.
///
/// The API is intentionally plain HTTP — secure it via an SSH tunnel.
/// Folder mappings restrict which directories can be accessed via the API.
///
/// Example:
///   rgringotts -p 7979 -h 127.0.0.1 -f mydata=/home/user/.gringotts
#[derive(Parser)]
#[command(name = "rgringotts")]
struct Cli {
    /// Path to a TOML configuration file.
    /// Defaults to `./rgringotts.toml` if it exists.
    #[arg(short = 'c', long = "config", value_name = "FILE")]
    config: Option<PathBuf>,

    /// TCP port to listen on (overrides config file).
    #[arg(short = 'p', long = "port", value_name = "PORT")]
    port: Option<u16>,

    /// Host / IP address to bind to (overrides config file).
    #[arg(short = 'h', long = "host", value_name = "HOST")]
    host: Option<String>,

    /// Add a folder mapping NAME=/path/to/dir (repeatable, merges with config file).
    /// Clients use `NAME:///filename` as the file specifier in API calls.
    #[arg(short = 'f', long = "folder", value_name = "NAME=PATH")]
    folders: Vec<String>,
}

fn parse_folder_arg(s: &str) -> Result<(String, PathBuf), String> {
    s.find('=')
        .map(|i| (s[..i].to_owned(), PathBuf::from(&s[i + 1..])))
        .ok_or_else(|| format!("Expected NAME=PATH, got '{s}'"))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rgringotts=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    // 1. Start with an empty config.
    let mut cfg = config::Config::default();

    // 2. Load the config file (explicit path, or default ./rgringotts.toml).
    let cfg_path = cli
        .config
        .clone()
        .unwrap_or_else(|| PathBuf::from("rgringotts.toml"));

    if cfg_path.exists() {
        match config::Config::load(&cfg_path) {
            Ok(file_cfg) => cfg.merge(file_cfg),
            Err(e) => {
                eprintln!("Error loading config: {e}");
                std::process::exit(1);
            }
        }
    } else if cli.config.is_some() {
        eprintln!("Config file not found: {}", cfg_path.display());
        std::process::exit(1);
    }

    // 3. CLI overrides.
    if let Some(p) = cli.port {
        cfg.port = Some(p);
    }
    if let Some(h) = cli.host {
        cfg.host = Some(h);
    }
    let mut cli_folders: HashMap<String, PathBuf> = HashMap::new();
    for raw in &cli.folders {
        match parse_folder_arg(raw) {
            Ok((name, path)) => {
                cli_folders.insert(name, path);
            }
            Err(e) => {
                eprintln!("Invalid --folder argument: {e}");
                std::process::exit(1);
            }
        }
    }
    cfg.folders.extend(cli_folders);

    let port = cfg.port.unwrap_or(7979);
    let host = cfg.host.as_deref().unwrap_or("127.0.0.1").to_owned();

    if cfg.folders.is_empty() {
        tracing::warn!("No folder mappings configured — any absolute path can be opened.");
    } else {
        for (name, path) in &cfg.folders {
            tracing::info!("Folder mapping: {name} → {}", path.display());
        }
    }

    let state = Arc::new(AppState::new(cfg.folders));

    // Background task: evict sessions that have exceeded the 30-second timeout.
    {
        let sessions = Arc::clone(&state.sessions);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                sessions.expire_old();
            }
        });
    }

    let app = routes::build_router(state);

    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("Invalid bind address");

    tracing::info!("Listening on http://{addr}");
    tracing::info!("Use an SSH tunnel — this service has no TLS by design.");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

