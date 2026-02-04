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

#[tokio::test]
async fn settlements_are_validated_and_can_be_undone() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Ann").await;
    let (group, ann) = app.group(&owner, "Flat", "EUR").await;
    let ben = app.placeholder(&owner, &group, "Ben").await;
    let uri = format!("/api/groups/{group}/settlements");

    let invalid = app
        .post(
            &uri,
            &owner.token,
            json!({ "from_member": ann, "to_member": ann, "amount_cents": -5, "settled_on": common::today() }),
        )
        .await;
    assert_eq!(invalid.status, StatusCode::UNPROCESSABLE_ENTITY);
    let fields = &invalid.json()["error"]["fields"];
    assert!(fields["to_member"].is_string());
    assert!(fields["amount_cents"].is_string());

    let created = app
        .post(
            &uri,
            &owner.token,
            json!({ "from_member": ben, "to_member": ann, "amount_cents": 2_500, "settled_on": common::today(), "note": "cash" }),
        )
        .await
        .json();
    let balances = app
        .get(&format!("/api/groups/{group}/balances"), &owner.token)
        .await
        .json();
    assert_eq!(net_of(&balances, &ben), 2_500);

    let id = created["id"].as_str().unwrap();
    assert_eq!(
        app.delete(&format!("{uri}/{id}"), &owner.token)
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert!(
        app.get(&uri, &owner.token)
            .await
            .json()
            .as_array()
            .unwrap()
            .is_empty()
    );

    let feed = app
        .get(&format!("/api/groups/{group}/activity"), &owner.token)
        .await
        .json();
    assert_eq!(feed["items"][0]["kind"], "settlement_deleted");
    assert_eq!(feed["items"][0]["payload"]["from"], "Ben");
}

#[tokio::test]
async fn activity_feed_pages_backwards() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Ann").await;
    let (group, ann) = app.group(&owner, "Flat", "EUR").await;
    for i in 0..4 {
        app.equal_expense(&owner, &group, &format!("Thing {i}"), 100, &ann, &[&ann])
            .await;
    }
    let uri = format!("/api/groups/{group}/activity");

    let first = app
        .get(&format!("{uri}?limit=3"), &owner.token)
        .await
        .json();
    assert_eq!(first["items"].as_array().unwrap().len(), 3);
    assert_eq!(first["items"][0]["payload"]["description"], "Thing 3");
    assert_eq!(first["items"][0]["actor_name"], "Ann");

    let before = first["next_before"].as_str().unwrap();
    let second = app
        .get(
            &format!("{uri}?limit=3&before={}", before.replace('+', "%2B")),
            &owner.token,
        )
        .await
        .json();
    let kinds: Vec<&str> = second["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, vec!["expense_added", "group_created"]);
    assert!(second["next_before"].is_null());

    // Exactly one full page: no cursor to an empty next page.
    let all = app
        .get(&format!("{uri}?limit=5"), &owner.token)
        .await
        .json();
    assert_eq!(all["items"].as_array().unwrap().len(), 5);
    assert!(all["next_before"].is_null());
}
