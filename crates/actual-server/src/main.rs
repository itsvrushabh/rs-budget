//! `actual-server` binary entrypoint.

use actual_server::{create_app, ServerState};
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(
    name = "actual-server",
    about = "100% Rust self-hosted sync server for Actual Budget",
    version
)]
struct Args {
    /// Port number to listen on
    #[arg(short, long, env = "PORT", default_value = "5006")]
    port: u16,

    /// Host IP address to bind to
    #[arg(long, env = "HOST", default_value = "0.0.0.0")]
    host: String,

    /// Directory for database and uploaded budget archives
    #[arg(long, env = "ACTUAL_DATA_DIR", default_value = "./data")]
    data_dir: PathBuf,

    /// Server master password (optional, can also be bootstrapped via web UI)
    #[arg(long, env = "ACTUAL_SERVER_PASSWORD")]
    server_password: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "actual_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    tracing::info!(
        data_dir = %args.data_dir.display(),
        "Initializing actual-server storage..."
    );

    let state = ServerState::new(&args.data_dir, args.server_password)?;
    let app = create_app(state);

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    tracing::info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
