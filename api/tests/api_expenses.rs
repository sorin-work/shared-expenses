mod common;

use axum::http::StatusCode;
use serde_json::{Value, json};

fn amounts(expense: &Value) -> Vec<i64> {
    expense["shares"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["amount_cents"].as_i64().unwrap())
        .collect()
}

#[tokio::test]
async fn other_split_methods() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Owner").await;
    let (group, me) = app.group(&owner, "Trip", "EUR").await;
    let b = app.placeholder(&owner, &group, "B").await;
    let uri = format!("/api/groups/{group}/expenses");

    let base = |method: &str, splits: Value| {
        json!({
            "description": "Hotel", "amount_cents": 10_001, "paid_by": me,
            "split_method": method, "category": "accommodation",
            "spent_on": common::today(), "splits": splits,
        })
    };

    let exact = app
        .post(
            &uri,
            &owner.token,
            base(
                "exact",
                json!([{ "member_id": me, "value": 6_001 }, { "member_id": b, "value": 4_000 }]),
            ),
        )
        .await;
    assert_eq!(exact.status, StatusCode::CREATED, "{}", exact.text);
    assert_eq!(amounts(&exact.json()), vec![6_001, 4_000]);

    let percent = app
        .post(
            &uri,
            &owner.token,
            base(
                "percent",
                json!([{ "member_id": me, "value": 2_500 }, { "member_id": b, "value": 7_500 }]),
            ),
        )
        .await;
    assert_eq!(amounts(&percent.json()), vec![2_500, 7_501]);
    assert_eq!(percent.json()["shares"][1]["input"], 7_500);

    let shares = app
        .post(
            &uri,
            &owner.token,
            base(
                "shares",
                json!([{ "member_id": me, "value": 1 }, { "member_id": b, "value": 2 }]),
            ),
        )
        .await;
    assert_eq!(amounts(&shares.json()), vec![3_334, 6_667]);
}
