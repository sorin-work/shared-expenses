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
async fn equal_split_hands_leftover_cents_out_in_member_order() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Owner").await;
    let (group, me) = app.group(&owner, "Flat", "EUR").await;
    let b = app.placeholder(&owner, &group, "B").await;
    let c = app.placeholder(&owner, &group, "C").await;

    // Listed in reverse, but the first member still gets the extra cent.
    let expense = app
        .equal_expense(&owner, &group, "Pizza", 1_000, &me, &[&c, &b, &me])
        .await;
    assert_eq!(amounts(&expense), vec![334, 333, 333]);
    assert_eq!(expense["shares"][0]["member_id"], me.as_str());
    assert_eq!(expense["category"], "general");
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

#[tokio::test]
async fn accepts_tomorrow_for_timezones_ahead_of_utc() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Owner").await;
    let (group, me) = app.group(&owner, "Auckland", "NZD").await;
    let tomorrow = (chrono::Utc::now().date_naive() + chrono::Days::new(1)).to_string();
    let response = app
        .post(
            &format!("/api/groups/{group}/expenses"),
            &owner.token,
            json!({
                "description": "Breakfast", "amount_cents": 1_850, "paid_by": me,
                "split_method": "equal", "spent_on": tomorrow, "splits": [{ "member_id": me }],
            }),
        )
        .await;
    assert_eq!(response.status, StatusCode::CREATED, "{}", response.text);
}

#[tokio::test]
async fn edits_are_kept_in_the_history() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Owner").await;
    let (group, me) = app.group(&owner, "Flat", "EUR").await;
    let b = app.placeholder(&owner, &group, "B").await;
    let expense = app
        .equal_expense(&owner, &group, "Groceries", 2_000, &me, &[&me, &b])
        .await;
    let id = expense["id"].as_str().unwrap();
    let uri = format!("/api/groups/{group}/expenses/{id}");

    let edit = json!({
        "description": "Groceries", "amount_cents": 2_400, "paid_by": me,
        "split_method": "equal", "spent_on": common::today(), "notes": "forgot the wine",
        "splits": [{ "member_id": me }, { "member_id": b }],
    });
    let updated = app.put(&uri, &owner.token, edit.clone()).await;
    assert_eq!(updated.status, StatusCode::OK, "{}", updated.text);
    assert_eq!(amounts(&updated.json()), vec![1_200, 1_200]);

    // Saving the same thing again doesn't add a revision.
    app.put(&uri, &owner.token, edit).await;

    let history = app
        .get(&format!("{uri}/history"), &owner.token)
        .await
        .json();
    let revisions = history.as_array().unwrap();
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0]["action"], "created");
    assert_eq!(revisions[0]["snapshot"]["amount_cents"], 2_000);
    assert_eq!(revisions[1]["action"], "updated");
    assert_eq!(revisions[1]["snapshot"]["notes"], "forgot the wine");
    assert_eq!(revisions[1]["changed_by_name"], "Owner");

    let feed = app
        .get(&format!("/api/groups/{group}/activity"), &owner.token)
        .await
        .json();
    assert_eq!(feed["items"][0]["kind"], "expense_updated");
    assert_eq!(
        feed["items"][0]["payload"]["changes"],
        json!(["amount", "notes", "split"])
    );
}

#[tokio::test]
async fn deleted_expenses_disappear_but_keep_their_history() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Owner").await;
    let (group, me) = app.group(&owner, "Flat", "EUR").await;
    let b = app.placeholder(&owner, &group, "B").await;
    let expense = app
        .equal_expense(&owner, &group, "Oops", 5_000, &me, &[&me, &b])
        .await;
    let id = expense["id"].as_str().unwrap();
    let uri = format!("/api/groups/{group}/expenses/{id}");

    assert_eq!(
        app.delete(&uri, &owner.token).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.get(&uri, &owner.token).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.delete(&uri, &owner.token).await.status,
        StatusCode::NOT_FOUND
    );

    let list = app
        .get(&format!("/api/groups/{group}/expenses"), &owner.token)
        .await
        .json();
    assert_eq!(list["total"], 0);
    let balances = app
        .get(&format!("/api/groups/{group}/balances"), &owner.token)
        .await
        .json();
    assert_eq!(balances["balances"][0]["net"], 0);

    let history = app
        .get(&format!("{uri}/history"), &owner.token)
        .await
        .json();
    assert_eq!(
        history.as_array().unwrap().last().unwrap()["action"],
        "deleted"
    );
}

#[tokio::test]
async fn listing_filters_and_paginates() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Owner").await;
    let (group, me) = app.group(&owner, "Flat", "EUR").await;
    let b = app.placeholder(&owner, &group, "B").await;
    for description in ["Rent", "Groceries 50% off", "Groceries"] {
        app.equal_expense(&owner, &group, description, 1_000, &me, &[&me])
            .await;
    }
    app.equal_expense(&owner, &group, "Taxi", 1_000, &me, &[&b])
        .await;
    let uri = format!("/api/groups/{group}/expenses");

    let all = app.get(&uri, &owner.token).await.json();
    assert_eq!(all["total"], 4);

    let search = app
        .get(&format!("{uri}?q=groceries"), &owner.token)
        .await
        .json();
    assert_eq!(search["total"], 2);

    // % is matched literally, not as a wildcard.
    let percent = app
        .get(&format!("{uri}?q=50%25"), &owner.token)
        .await
        .json();
    assert_eq!(percent["total"], 1);

    let for_b = app
        .get(&format!("{uri}?member={b}"), &owner.token)
        .await
        .json();
    assert_eq!(for_b["total"], 1);
    assert_eq!(for_b["items"][0]["description"], "Taxi");

    let page = app
        .get(&format!("{uri}?limit=3&offset=3"), &owner.token)
        .await
        .json();
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(page["total"], 4);

    let none = app
        .get(&format!("{uri}?category=rent"), &owner.token)
        .await
        .json();
    assert_eq!(none["total"], 0);
}
