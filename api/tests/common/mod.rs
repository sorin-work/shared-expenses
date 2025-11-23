//! Helpers for the API tests. Tests that need a database are skipped unless
//! TEST_DATABASE_URL points at a Postgres database they may write to.

#![allow(dead_code)]

use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use shared_expenses_api::auth::token::TokenKeys;
use shared_expenses_api::router;
use shared_expenses_api::state::AppState;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

pub const SECRET: &str = "integration-test-secret-0123456789";

pub struct Response {
    pub status: StatusCode,
    pub headers: axum::http::HeaderMap,
    pub text: String,
}

impl Response {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.text).unwrap_or_else(|e| panic!("not json ({e}): {}", self.text))
    }
}

/// An app whose database is never reachable, for tests of everything that
/// happens before a query is made.
pub fn offline_app() -> Router {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(300))
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none")
        .expect("lazy pool");
    router(AppState::new(pool, TokenKeys::new(SECRET, 1)))
}

pub async fn send(
    router: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> Response {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = match body {
        Some(body) => request
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    }
    .expect("request");

    let response = router.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    Response {
        status,
        headers,
        text: String::from_utf8(bytes.to_vec()).expect("utf-8 body"),
    }
}
