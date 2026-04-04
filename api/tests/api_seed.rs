mod common;

use axum::http::StatusCode;
use common::send;
use serde_json::json;
use shared_expenses_api::seed;

#[tokio::test]
async fn demo_data_is_consistent() {
    let Some(app) = common::app().await else {
        return;
    };
    // Running it twice must not duplicate anything.
    seed::run(&app.pool).await.expect("seed");
    seed::run(&app.pool).await.expect("seed again");

    let login = send(
        &app.router,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "email": seed::DEMO_EMAIL, "password": seed::DEMO_PASSWORD })),
    )
    .await;
    assert_eq!(login.status, StatusCode::OK);
    let token = login.json()["token"].as_str().unwrap().to_string();

    let groups = app.get("/api/groups", &token).await.json();
    let groups = groups.as_array().unwrap();
    assert_eq!(groups.len(), 3);

    for group in groups {
        let id = group["id"].as_str().unwrap();
        let balances = app
            .get(&format!("/api/groups/{id}/balances"), &token)
            .await
            .json();
        let total: i64 = balances["balances"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["net"].as_i64().unwrap())
            .sum();
        assert_eq!(total, 0, "{}", group["name"]);
        assert!(!balances["suggestions"].as_array().unwrap().is_empty());

        let expenses = app
            .get(&format!("/api/groups/{id}/expenses"), &token)
            .await
            .json();
        assert!(expenses["total"].as_i64().unwrap() >= 10);
    }
}
