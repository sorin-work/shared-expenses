//! shared-expenses API: groups, expenses, balances and settling up.

pub mod config;
pub mod error;
pub mod routes;
pub mod state;

use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub use routes::router;

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}
