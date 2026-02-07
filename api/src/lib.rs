//! shared-expenses API: groups, expenses, balances and settling up.

pub mod access;
pub mod activity;
pub mod auth;
pub mod config;
pub mod csv;
pub mod domain;
pub mod error;
pub mod extract;
pub mod routes;
pub mod state;
pub mod validate;

use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub use routes::router;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}
