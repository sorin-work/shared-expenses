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

#[tokio::test]
async fn invalid_splits_are_reported_on_the_splits_field() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Owner").await;
    let (group, me) = app.group(&owner, "Trip", "EUR").await;
    let other_owner = app.signup("Other").await;
    let (_, stranger) = app.group(&other_owner, "Elsewhere", "EUR").await;
    let uri = format!("/api/groups/{group}/expenses");

    let cases = [
        (
            "exact",
            json!([{ "member_id": me, "value": 999 }]),
            "splits",
        ),
        (
            "percent",
            json!([{ "member_id": me, "value": 9_000 }]),
            "splits",
        ),
        ("shares", json!([{ "member_id": me, "value": 0 }]), "splits"),
        ("equal", json!([]), "splits"),
        ("equal", json!([{ "member_id": stranger }]), "splits"),
        ("thirds", json!([{ "member_id": me }]), "split_method"),
    ];
    for (method, splits, field) in cases {
        let response = app
            .post(
                &uri,
                &owner.token,
                json!({
                    "description": "Thing", "amount_cents": 1_000, "paid_by": me,
                    "split_method": method, "spent_on": common::today(), "splits": splits,
                }),
            )
            .await;
        assert_eq!(
            response.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{method}: {}",
            response.text
        );
        assert!(
            response.json()["error"]["fields"][field].is_string(),
            "{method}: {}",
            response.text
        );
    }
}

#[tokio::test]
async fn basic_fields_are_validated() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Owner").await;
    let (group, me) = app.group(&owner, "Trip", "EUR").await;
    let response = app
        .post(
            &format!("/api/groups/{group}/expenses"),
            &owner.token,
            json!({
                "description": "  ", "amount_cents": 0, "paid_by": me, "split_method": "equal",
                "category": "yachts", "spent_on": "2999-01-01", "splits": [{ "member_id": me }],
            }),
        )
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    let fields = &response.json()["error"]["fields"];
    for field in ["description", "amount_cents", "category", "spent_on"] {
        assert!(fields[field].is_string(), "{field} in {fields}");
    }
}
