//! Handler tests that don't need a database: routing, authentication and
//! input validation all happen before any query.

mod common;

use axum::http::StatusCode;
use common::{SECRET, offline_app, send};
use serde_json::json;
use shared_expenses_api::auth::token::TokenKeys;
use uuid::Uuid;

#[tokio::test]
async fn unknown_routes_return_json_404() {
    let response = send(&offline_app(), "GET", "/api/nope", None, None).await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_eq!(response.json()["error"]["code"], "not_found");
}

#[tokio::test]
async fn protected_routes_require_a_token() {
    for (method, uri) in [
        ("GET", "/api/auth/me"),
        ("GET", "/api/groups"),
        ("POST", "/api/groups"),
        ("GET", "/api/invites/abc"),
    ] {
        let response = send(&offline_app(), method, uri, None, None).await;
        assert_eq!(response.status, StatusCode::UNAUTHORIZED, "{method} {uri}");
        assert_eq!(response.json()["error"]["code"], "unauthorized");
    }
}

#[tokio::test]
async fn invalid_and_foreign_tokens_are_rejected() {
    let foreign = TokenKeys::new("some-other-secret-some-other-secret", 1)
        .issue(Uuid::new_v4())
        .unwrap();
    for token in ["garbage", foreign.as_str()] {
        let response = send(&offline_app(), "GET", "/api/auth/me", Some(token), None).await;
        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn signup_validates_every_field() {
    let response = send(
        &offline_app(),
        "POST",
        "/api/auth/signup",
        None,
        Some(json!({ "email": "not-an-email", "name": " ", "password": "short" })),
    )
    .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    let fields = &response.json()["error"]["fields"];
    assert_eq!(fields["email"], "is not a valid email address");
    assert_eq!(fields["name"], "is required");
    assert_eq!(fields["password"], "must be at least 8 characters");
}

#[tokio::test]
async fn login_requires_email_and_password() {
    let response = send(
        &offline_app(),
        "POST",
        "/api/auth/login",
        None,
        Some(json!({})),
    )
    .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    let fields = &response.json()["error"]["fields"];
    assert_eq!(fields["email"], "is required");
    assert_eq!(fields["password"], "is required");
}

#[tokio::test]
async fn malformed_json_is_a_bad_request() {
    let response = send(
        &offline_app(),
        "POST",
        "/api/auth/login",
        None,
        Some(json!("just a string")),
    )
    .await;
    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert_eq!(response.json()["error"]["code"], "bad_request");
}

#[tokio::test]
async fn invalid_ids_in_the_path_are_not_found() {
    let token = TokenKeys::new(SECRET, 1).issue(Uuid::new_v4()).unwrap();
    let response = send(
        &offline_app(),
        "GET",
        "/api/groups/not-a-uuid",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
}
