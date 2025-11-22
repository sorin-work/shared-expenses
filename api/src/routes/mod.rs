use axum::Router;
use axum::routing::get;

use crate::state::AppState;

pub mod auth;
pub mod health;

pub fn router(state: AppState) -> Router {
    let api = Router::new().nest("/auth", auth::router());

    Router::new()
        .route("/healthz", get(health::check))
        .nest("/api", api)
        .with_state(state)
}
