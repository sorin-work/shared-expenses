mod common;

use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn creating_a_group_makes_the_creator_its_owner() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Owner").await;
    let (group, member) = app.group(&owner, "Flat 2", "eur").await;

    let detail = app
        .get(&format!("/api/groups/{group}"), &owner.token)
        .await
        .json();
    assert_eq!(detail["currency"], "EUR");
    assert_eq!(detail["my_role"], "owner");
    assert_eq!(detail["members"][0]["id"], member.as_str());
    assert_eq!(detail["members"][0]["display_name"], "Owner");
    assert_eq!(detail["invite_code"].as_str().unwrap().len(), 10);

    let list = app.get("/api/groups", &owner.token).await.json();
    let entry = list
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == group.as_str())
        .unwrap();
    assert_eq!(entry["member_count"], 1);
    assert_eq!(entry["my_balance"], 0);
}

#[tokio::test]
async fn group_input_is_validated() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Owner").await;
    let response = app
        .post(
            "/api/groups",
            &owner.token,
            json!({ "name": "", "currency": "XYZ" }),
        )
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    let fields = &response.json()["error"]["fields"];
    assert_eq!(fields["name"], "is required");
    assert_eq!(fields["currency"], "is not a supported currency");
}

#[tokio::test]
async fn only_the_owner_can_change_settings() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Owner").await;
    let friend = app.signup("Friend").await;
    let (group, _) = app.group(&owner, "Trip", "EUR").await;
    let code = app
        .get(&format!("/api/groups/{group}"), &owner.token)
        .await
        .json()["invite_code"]
        .as_str()
        .unwrap()
        .to_string();
    app.post(
        &format!("/api/invites/{code}/join"),
        &friend.token,
        json!({}),
    )
    .await;

    let forbidden = app
        .patch(
            &format!("/api/groups/{group}"),
            &friend.token,
            json!({ "name": "Mine" }),
        )
        .await;
    assert_eq!(forbidden.status, StatusCode::FORBIDDEN);

    let renamed = app
        .patch(
            &format!("/api/groups/{group}"),
            &owner.token,
            json!({ "name": "Rome trip" }),
        )
        .await;
    assert_eq!(renamed.status, StatusCode::OK);
    assert_eq!(renamed.json()["name"], "Rome trip");

    let deleted = app
        .delete(&format!("/api/groups/{group}"), &friend.token)
        .await;
    assert_eq!(deleted.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn currency_is_locked_once_there_are_expenses() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Owner").await;
    let (group, me) = app.group(&owner, "Home", "EUR").await;

    let changed = app
        .patch(
            &format!("/api/groups/{group}"),
            &owner.token,
            json!({ "currency": "GBP" }),
        )
        .await;
    assert_eq!(changed.json()["currency"], "GBP");

    app.equal_expense(&owner, &group, "Milk", 120, &me, &[&me])
        .await;
    let blocked = app
        .patch(
            &format!("/api/groups/{group}"),
            &owner.token,
            json!({ "currency": "EUR" }),
        )
        .await;
    assert_eq!(blocked.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn joining_with_the_invite_link() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Owner").await;
    let friend = app.signup("Owner").await;
    let (group, _) = app.group(&owner, "Holiday", "EUR").await;
    let code = app
        .get(&format!("/api/groups/{group}"), &owner.token)
        .await
        .json()["invite_code"]
        .as_str()
        .unwrap()
        .to_string();

    let preview = app
        .get(&format!("/api/invites/{code}"), &friend.token)
        .await;
    assert_eq!(preview.status, StatusCode::OK);
    assert_eq!(preview.json()["name"], "Holiday");
    assert_eq!(preview.json()["already_member"], false);

    let joined = app
        .post(
            &format!("/api/invites/{code}/join"),
            &friend.token,
            json!({}),
        )
        .await;
    assert_eq!(joined.status, StatusCode::OK, "{}", joined.text);

    // Same name as the owner, so it gets a number.
    let detail = app
        .get(&format!("/api/groups/{group}"), &friend.token)
        .await
        .json();
    let names: Vec<&str> = detail["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["display_name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["Owner", "Owner 2"]);

    // Joining again changes nothing.
    let again = app
        .post(
            &format!("/api/invites/{code}/join"),
            &friend.token,
            json!({}),
        )
        .await;
    assert_eq!(again.json()["member_id"], joined.json()["member_id"]);

    let unknown = app.get("/api/invites/doesnotexist", &friend.token).await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn regenerating_the_invite_invalidates_the_old_link() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Owner").await;
    let friend = app.signup("Friend").await;
    let (group, _) = app.group(&owner, "Flat", "EUR").await;
    let old = app
        .get(&format!("/api/groups/{group}"), &owner.token)
        .await
        .json()["invite_code"]
        .as_str()
        .unwrap()
        .to_string();

    let new = app
        .post(
            &format!("/api/groups/{group}/invite"),
            &owner.token,
            json!({}),
        )
        .await
        .json();
    assert_ne!(new["invite_code"], old.as_str());
    let response = app
        .post(
            &format!("/api/invites/{old}/join"),
            &friend.token,
            json!({}),
        )
        .await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn claiming_a_placeholder_keeps_its_expenses() {
    let Some(app) = common::app().await else {
        return;
    };
    let owner = app.signup("Owner").await;
    let friend = app.signup("Chris").await;
    let (group, me) = app.group(&owner, "Ski week", "CHF").await;
    let placeholder = app.placeholder(&owner, &group, "Chris").await;
    app.equal_expense(
        &owner,
        &group,
        "Lift passes",
        30_000,
        &me,
        &[&me, &placeholder],
    )
    .await;

    let code = app
        .get(&format!("/api/groups/{group}"), &owner.token)
        .await
        .json()["invite_code"]
        .as_str()
        .unwrap()
        .to_string();
    let preview = app
        .get(&format!("/api/invites/{code}"), &friend.token)
        .await
        .json();
    assert_eq!(preview["placeholders"][0]["id"], placeholder.as_str());

    let joined = app
        .post(
            &format!("/api/invites/{code}/join"),
            &friend.token,
            json!({ "claim_member_id": placeholder }),
        )
        .await;
    assert_eq!(joined.json()["member_id"], placeholder.as_str());

    let groups = app.get("/api/groups", &friend.token).await.json();
    assert_eq!(groups[0]["my_balance"], -15_000);
}
