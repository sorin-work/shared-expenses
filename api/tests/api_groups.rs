mod common;

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
}
