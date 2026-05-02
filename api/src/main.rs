use std::net::SocketAddr;
use std::time::Duration;

use shared_expenses_api::auth::token::TokenKeys;
use shared_expenses_api::config::Config;
use shared_expenses_api::state::AppState;
use shared_expenses_api::{MIGRATOR, connect, router, seed};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // `api healthcheck` is used by the Docker healthcheck, so the runtime
    // image doesn't need curl or wget.
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        std::process::exit(if healthcheck().await { 0 } else { 1 });
    }

    if let Err(err) = run().await {
        tracing::error!(error = %err, "fatal");
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("shared_expenses_api=info,tower_http=info"));
    let json = std::env::var("LOG_FORMAT").is_ok_and(|f| f.eq_ignore_ascii_case("json"));
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    if json {
        builder.json().init();
    } else {
        builder.init();
    }
}

async fn healthcheck() -> bool {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let port = std::env::var("PORT").unwrap_or_else(|_| "4407".into());
    let check = async {
        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}")).await?;
        stream
            .write_all(b"GET /healthz HTTP/1.0\r\nHost: localhost\r\n\r\n")
            .await?;
        let mut buf = vec![0u8; 64];
        let n = stream.read(&mut buf).await?;
        Ok::<_, std::io::Error>(
            buf[..n].starts_with(b"HTTP/1.1 200") || buf[..n].starts_with(b"HTTP/1.0 200"),
        )
    };
    matches!(
        tokio::time::timeout(Duration::from_secs(3), check).await,
        Ok(Ok(true))
    )
}

/// Postgres may still be starting when the API comes up, so retry for a bit.
async fn connect_with_retry(url: &str) -> Result<sqlx::PgPool, sqlx::Error> {
    let mut attempt = 1;
    loop {
        match connect(url).await {
            Ok(pool) => return Ok(pool),
            Err(err) if attempt < 10 => {
                tracing::warn!(error = %err, attempt, "database not reachable yet");
                tokio::time::sleep(Duration::from_secs(2)).await;
                attempt += 1;
            }
            Err(err) => return Err(err),
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let config = Config::from_env()?;

    let pool = connect_with_retry(&config.database_url).await?;
    MIGRATOR.run(&pool).await?;
    tracing::info!("migrations applied");

    if config.seed_demo {
        seed::run(&pool).await?;
    }

    let state = AppState::new(
        pool,
        TokenKeys::new(&config.jwt_secret, config.token_ttl_hours),
    );
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "listening");

    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
    tracing::info!("shutting down");
}
