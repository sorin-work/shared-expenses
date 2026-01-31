use axum::Router;
use axum::routing::get;
use tower_http::trace::TraceLayer;

use crate::error::AppError;
use crate::state::AppState;

pub mod activity;
pub mod auth;
pub mod balances;
pub mod expenses;
pub mod groups;
pub mod health;
pub mod invites;
pub mod members;
pub mod settlements;

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .nest("/auth", auth::router())
        .nest("/groups", groups::router())
        .nest("/groups/{group_id}/members", members::router())
        .nest("/groups/{group_id}/expenses", expenses::router())
        .nest("/groups/{group_id}/settlements", settlements::router())
        .route("/groups/{group_id}/balances", get(balances::show))
        .route("/groups/{group_id}/activity", get(activity::list))
        .nest("/invites", invites::router());

    Router::new()
        .route("/healthz", get(health::check))
        .nest("/api", api)
        .fallback(not_found)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn not_found() -> AppError {
    AppError::NotFound("endpoint")
}
