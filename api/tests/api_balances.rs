mod common;

use axum::http::StatusCode;
use serde_json::{Value, json};

fn net_of(balances: &Value, member: &str) -> i64 {
    balances["balances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["member_id"] == member)
        .unwrap()["net"]
        .as_i64()
        .unwrap()
}

#[tokio::test]
async fn balances_and_settle_up_suggestions() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Ann").await;
    let (group, ann) = app.group(&owner, "Flat", "EUR").await;
    let ben = app.placeholder(&owner, &group, "Ben").await;
    let cat = app.placeholder(&owner, &group, "Cat").await;

    app.equal_expense(&owner, &group, "Rent", 90_000, &ann, &[&ann, &ben, &cat])
        .await;
    app.equal_expense(&owner, &group, "Food", 3_000, &ben, &[&ann, &ben, &cat])
        .await;

    let uri = format!("/api/groups/{group}/balances");
    let balances = app.get(&uri, &owner.token).await.json();
    assert_eq!(balances["currency"], "EUR");
    assert_eq!(net_of(&balances, &ann), 59_000);
    assert_eq!(net_of(&balances, &ben), -28_000);
    assert_eq!(net_of(&balances, &cat), -31_000);
    let suggestions = balances["suggestions"].as_array().unwrap();
    assert_eq!(suggestions.len(), 2);
    assert!(suggestions.iter().all(|s| s["to_member"] == ann.as_str()));

    // Paying back what was suggested settles everyone.
    for s in suggestions {
        let response = app
            .post(
                &format!("/api/groups/{group}/settlements"),
                &owner.token,
                json!({
                    "from_member": s["from_member"], "to_member": s["to_member"],
                    "amount_cents": s["amount_cents"], "settled_on": common::today(),
                }),
            )
            .await;
        assert_eq!(response.status, StatusCode::CREATED, "{}", response.text);
    }
    let settled = app.get(&uri, &owner.token).await.json();
    assert!(
        settled["balances"]
            .as_array()
            .unwrap()
            .iter()
            .all(|b| b["net"] == 0)
    );
    assert!(settled["suggestions"].as_array().unwrap().is_empty());
}
