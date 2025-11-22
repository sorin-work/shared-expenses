use std::net::SocketAddr;

use shared_expenses_api::auth::token::TokenKeys;
use shared_expenses_api::config::Config;
use shared_expenses_api::state::AppState;
use shared_expenses_api::{MIGRATOR, connect, router};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        tracing::error!(error = %err, "fatal");
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("shared_expenses_api=info,tower_http=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let config = Config::from_env()?;

    let pool = connect(&config.database_url).await?;
    MIGRATOR.run(&pool).await?;
    tracing::info!("migrations applied");

    let state = AppState::new(pool, TokenKeys::new(&config.jwt_secret, 24 * 7));
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "listening");

    axum::serve(listener, router(state)).await?;
    Ok(())
}
