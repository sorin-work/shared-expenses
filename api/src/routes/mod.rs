use axum::Router;
use axum::routing::get;

use crate::error::AppError;
use crate::state::AppState;

pub mod auth;
pub mod health;

pub fn router(state: AppState) -> Router {
    let api = Router::new().nest("/auth", auth::router());

    Router::new()
        .route("/healthz", get(health::check))
        .nest("/api", api)
        .fallback(not_found)
        .with_state(state)
}

async fn not_found() -> AppError {
    AppError::NotFound("endpoint")
}
