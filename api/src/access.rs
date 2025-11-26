//! Checks that the signed-in user belongs to a group.

use sqlx::PgExecutor;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Membership {
    pub member_id: Uuid,
    pub role: String,
}

impl Membership {
    pub fn is_owner(&self) -> bool {
        self.role == "owner"
    }

    pub fn require_owner(&self) -> Result<(), AppError> {
        if self.is_owner() {
            Ok(())
        } else {
            Err(AppError::Forbidden("only the group owner can do that"))
        }
    }
}

/// Returns the user's membership in the group. Groups the user isn't part
/// of are reported as not found, so their existence isn't leaked.
pub async fn membership(
    db: impl PgExecutor<'_>,
    group_id: Uuid,
    user_id: Uuid,
) -> Result<Membership, AppError> {
    sqlx::query_as::<_, Membership>(
        "SELECT id AS member_id, role FROM members
         WHERE group_id = $1 AND user_id = $2 AND left_at IS NULL",
    )
    .bind(group_id)
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound("group"))
}
