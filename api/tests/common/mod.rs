//! Helpers for the API tests. Tests that need a database are skipped unless
//! TEST_DATABASE_URL points at a Postgres database they may write to.

#![allow(dead_code)]

use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use shared_expenses_api::auth::token::TokenKeys;
use shared_expenses_api::state::AppState;
use shared_expenses_api::{MIGRATOR, router};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

pub const SECRET: &str = "integration-test-secret-0123456789";

pub struct TestApp {
    pub router: Router,
    pub pool: PgPool,
}

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

/// Connects to TEST_DATABASE_URL and applies the migrations, or returns
/// `None` so the calling test can skip itself.
pub async fn app() -> Option<TestApp> {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("TEST_DATABASE_URL not set, skipping");
        return None;
    };
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("connect to TEST_DATABASE_URL");
    MIGRATOR.run(&pool).await.expect("migrations");
    let router = router(AppState::new(pool.clone(), TokenKeys::new(SECRET, 1)));
    Some(TestApp { router, pool })
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

pub struct Account {
    pub token: String,
    pub user_id: String,
    pub email: String,
}

impl TestApp {
    pub async fn get(&self, uri: &str, token: &str) -> Response {
        send(&self.router, "GET", uri, Some(token), None).await
    }

    pub async fn post(&self, uri: &str, token: &str, body: Value) -> Response {
        send(&self.router, "POST", uri, Some(token), Some(body)).await
    }

    pub async fn put(&self, uri: &str, token: &str, body: Value) -> Response {
        send(&self.router, "PUT", uri, Some(token), Some(body)).await
    }

    pub async fn patch(&self, uri: &str, token: &str, body: Value) -> Response {
        send(&self.router, "PATCH", uri, Some(token), Some(body)).await
    }

    pub async fn delete(&self, uri: &str, token: &str) -> Response {
        send(&self.router, "DELETE", uri, Some(token), None).await
    }

    /// Signs up a user with a unique email.
    pub async fn signup(&self, name: &str) -> Account {
        let email = format!("{}-{}@example.test", name.to_lowercase(), Uuid::new_v4());
        let response = send(
            &self.router,
            "POST",
            "/api/auth/signup",
            None,
            Some(json!({ "email": email, "name": name, "password": "password123" })),
        )
        .await;
        assert_eq!(response.status, StatusCode::CREATED, "{}", response.text);
        let body = response.json();
        Account {
            token: body["token"].as_str().unwrap().to_string(),
            user_id: body["user"]["id"].as_str().unwrap().to_string(),
            email,
        }
    }

    /// Creates a group and returns (group id, the owner's member id).
    pub async fn group(&self, owner: &Account, name: &str, currency: &str) -> (String, String) {
        let response = self
            .post(
                "/api/groups",
                &owner.token,
                json!({ "name": name, "currency": currency }),
            )
            .await;
        assert_eq!(response.status, StatusCode::CREATED, "{}", response.text);
        let body = response.json();
        (
            body["id"].as_str().unwrap().to_string(),
            body["my_member_id"].as_str().unwrap().to_string(),
        )
    }

    pub async fn placeholder(&self, owner: &Account, group: &str, name: &str) -> String {
        let response = self
            .post(
                &format!("/api/groups/{group}/members"),
                &owner.token,
                json!({ "display_name": name }),
            )
            .await;
        assert_eq!(response.status, StatusCode::CREATED, "{}", response.text);
        response.json()["id"].as_str().unwrap().to_string()
    }

    pub async fn equal_expense(
        &self,
        account: &Account,
        group: &str,
        description: &str,
        amount_cents: i64,
        paid_by: &str,
        members: &[&str],
    ) -> Value {
        let response = self
            .post(
                &format!("/api/groups/{group}/expenses"),
                &account.token,
                json!({
                    "description": description,
                    "amount_cents": amount_cents,
                    "paid_by": paid_by,
                    "split_method": "equal",
                    "spent_on": today(),
                    "splits": members.iter().map(|m| json!({ "member_id": m })).collect::<Vec<_>>(),
                }),
            )
            .await;
        assert_eq!(response.status, StatusCode::CREATED, "{}", response.text);
        response.json()
    }
}

pub fn today() -> String {
    chrono::Utc::now().date_naive().to_string()
}
