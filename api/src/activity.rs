//! The group activity feed. Entries carry a small JSON payload with the
//! names and amounts at the time, so the feed still reads correctly after
//! things are renamed or deleted.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgExecutor;
use uuid::Uuid;

pub mod kind {
    pub const GROUP_CREATED: &str = "group_created";
    pub const GROUP_RENAMED: &str = "group_renamed";
    pub const MEMBER_ADDED: &str = "member_added";
    pub const MEMBER_JOINED: &str = "member_joined";
    pub const MEMBER_RENAMED: &str = "member_renamed";
    pub const MEMBER_REMOVED: &str = "member_removed";
    pub const EXPENSE_ADDED: &str = "expense_added";
    pub const EXPENSE_UPDATED: &str = "expense_updated";
    pub const EXPENSE_DELETED: &str = "expense_deleted";
    pub const SETTLEMENT_ADDED: &str = "settlement_added";
    pub const SETTLEMENT_DELETED: &str = "settlement_deleted";
}

pub async fn record(
    db: impl PgExecutor<'_>,
    group_id: Uuid,
    actor_id: Uuid,
    kind: &str,
    payload: Value,
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO activity (id, group_id, actor_id, kind, payload, created_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(group_id)
    .bind(actor_id)
    .bind(kind)
    .bind(payload)
    .bind(at)
    .execute(db)
    .await?;
    Ok(())
}
