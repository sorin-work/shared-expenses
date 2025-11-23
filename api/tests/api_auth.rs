mod common;

use axum::http::StatusCode;
use common::send;
use serde_json::json;

#[tokio::test]
async fn signup_login_and_me() {
    let Some(app) = common::app().await else {
        return;
    };
    let account = app.signup("Robin").await;

    let me = app.get("/api/auth/me", &account.token).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.json()["name"], "Robin");
    assert_eq!(me.json()["email"], account.email);

    // Emails are matched case-insensitively.
    let login = send(
        &app.router,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "email": account.email.to_uppercase(), "password": "password123" })),
    )
    .await;
    assert_eq!(login.status, StatusCode::OK, "{}", login.text);
    assert_eq!(login.json()["user"]["id"], account.user_id.as_str());
    assert!(login.json()["user"].get("password_hash").is_none());
}

#[tokio::test]
async fn wrong_password_and_unknown_email_look_the_same() {
    let Some(app) = common::app().await else {
        return;
    };
    let account = app.signup("Kim").await;

    for (email, password) in [
        (account.email.as_str(), "wrong-password"),
        ("nobody-here@example.test", "password123"),
    ] {
        let response = send(
            &app.router,
            "POST",
            "/api/auth/login",
            None,
            Some(json!({ "email": email, "password": password })),
        )
        .await;
        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
        assert_eq!(response.json()["error"]["code"], "invalid_credentials");
    }
}
