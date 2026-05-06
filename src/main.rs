mod gringotts;
mod routes;
mod session;
mod state;

use std::{net::SocketAddr, sync::Arc};

use clap::Parser;
use tokio::time::Duration;
use tracing_subscriber::EnvFilter;

use state::AppState;

/// Remote gateway to a gringotts encrypted file.
///
/// The API is intentionally plain HTTP — secure it via an SSH tunnel.
///
/// Example:
///   rgringotts -p 7979 -h 127.0.0.1
#[derive(Parser)]
#[command(name = "rgringotts", disable_help_flag = true)]
struct Cli {
    /// TCP port to listen on (default 7979).
    #[arg(short = 'p', long = "port", default_value_t = 7979)]
    port: u16,

    /// Host / IP address to bind to (default 127.0.0.1).
    #[arg(short = 'h', long = "host", default_value = "127.0.0.1")]
    host: String,
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
    let state = Arc::new(AppState::new());

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

    let addr: SocketAddr = format!("{}:{}", cli.host, cli.port)
        .parse()
        .expect("Invalid bind address");

    tracing::info!("Listening on http://{addr}");
    tracing::info!("Use an SSH tunnel — this service has no TLS by design.");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

