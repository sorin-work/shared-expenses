use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MemberView {
    pub id: Uuid,
    pub display_name: String,
    pub role: String,
    /// False for placeholder people without an account.
    pub has_account: bool,
    pub joined_at: DateTime<Utc>,
}

/// All members of a group in joining order.
pub async fn list_members(
    db: impl PgExecutor<'_>,
    group_id: Uuid,
) -> Result<Vec<MemberView>, AppError> {
    Ok(sqlx::query_as::<_, MemberView>(
        "SELECT id, display_name, role, user_id IS NOT NULL AS has_account, joined_at
           FROM members WHERE group_id = $1
          ORDER BY joined_at, id",
    )
    .bind(group_id)
    .fetch_all(db)
    .await?)
}

pub async fn insert_member(
    db: impl PgExecutor<'_>,
    group_id: Uuid,
    user_id: Option<Uuid>,
    display_name: &str,
    role: &str,
    at: DateTime<Utc>,
) -> Result<Uuid, AppError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO members (id, group_id, user_id, display_name, role, joined_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(group_id)
    .bind(user_id)
    .bind(display_name)
    .bind(role)
    .bind(at)
    .execute(db)
    .await?;
    Ok(id)
}
