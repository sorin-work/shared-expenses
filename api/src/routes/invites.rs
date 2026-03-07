//! Joining a group through its invite link.

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::routing::{get, post};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use super::members;
use crate::activity::{self, kind};
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::extract::{Json, Path};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{code}", get(preview))
        .route("/{code}/join", post(join))
}

#[derive(sqlx::FromRow)]
struct InviteGroup {
    id: Uuid,
    name: String,
    currency: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Placeholder {
    pub id: Uuid,
    pub display_name: String,
}

#[derive(Serialize)]
pub struct InvitePreview {
    pub group_id: Uuid,
    pub name: String,
    pub currency: String,
    pub member_count: i64,
    pub already_member: bool,
    /// People without an account the new member can say they are.
    pub placeholders: Vec<Placeholder>,
}

#[derive(Deserialize, Default)]
pub struct JoinInput {
    pub claim_member_id: Option<Uuid>,
}

#[derive(Serialize)]
pub struct Joined {
    pub group_id: Uuid,
    pub member_id: Uuid,
}

async fn find_group(db: &mut PgConnection, code: &str) -> Result<InviteGroup, AppError> {
    sqlx::query_as::<_, InviteGroup>("SELECT id, name, currency FROM groups WHERE invite_code = $1")
        .bind(code.trim().to_ascii_lowercase())
        .fetch_optional(db)
        .await?
        .ok_or(AppError::NotFound("invite"))
}

async fn preview(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(code): Path<String>,
) -> Result<Json<InvitePreview>, AppError> {
    let mut conn = state.pool.acquire().await?;
    let group = find_group(&mut conn, &code).await?;

    let member_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM members WHERE group_id = $1 AND left_at IS NULL")
            .bind(group.id)
            .fetch_one(&mut *conn)
            .await?;
    let already_member: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM members
                         WHERE group_id = $1 AND user_id = $2 AND left_at IS NULL)",
    )
    .bind(group.id)
    .bind(auth.id)
    .fetch_one(&mut *conn)
    .await?;
    let placeholders = sqlx::query_as::<_, Placeholder>(
        "SELECT id, display_name FROM members
          WHERE group_id = $1 AND user_id IS NULL AND left_at IS NULL
          ORDER BY joined_at, id",
    )
    .bind(group.id)
    .fetch_all(&mut *conn)
    .await?;

    Ok(Json(InvitePreview {
        group_id: group.id,
        name: group.name,
        currency: group.currency,
        member_count,
        already_member,
        placeholders,
    }))
}

/// Picks a display name nobody active in the group uses yet: "Sam",
/// then "Sam 2", "Sam 3"...
async fn free_name(
    conn: &mut PgConnection,
    group_id: Uuid,
    wanted: &str,
) -> Result<String, AppError> {
    let taken: Vec<String> = sqlx::query_scalar(
        "SELECT lower(display_name) FROM members WHERE group_id = $1 AND left_at IS NULL",
    )
    .bind(group_id)
    .fetch_all(&mut *conn)
    .await?;
    Ok(pick_free_name(wanted, &taken))
}

fn pick_free_name(wanted: &str, taken_lowercase: &[String]) -> String {
    let is_taken = |name: &str| taken_lowercase.iter().any(|t| *t == name.to_lowercase());
    if !is_taken(wanted) {
        return wanted.to_string();
    }
    (2..)
        .map(|n| format!("{wanted} {n}"))
        .find(|candidate| !is_taken(candidate))
        .expect("there is always a free name")
}

async fn join(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(code): Path<String>,
    body: Bytes,
) -> Result<Json<Joined>, AppError> {
    // The body is optional: an empty POST joins as a new member.
    let input: JoinInput = if body.iter().all(u8::is_ascii_whitespace) {
        JoinInput::default()
    } else {
        serde_json::from_slice(&body).map_err(|e| AppError::BadRequest(e.to_string()))?
    };
    let user = super::auth::load_user(&state.pool, auth.id).await?;

    let mut tx = state.pool.begin().await?;
    let group = find_group(&mut tx, &code).await?;

    let existing: Option<(Uuid, bool)> = sqlx::query_as(
        "SELECT id, left_at IS NOT NULL FROM members WHERE group_id = $1 AND user_id = $2",
    )
    .bind(group.id)
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await?;

    let now = Utc::now();
    let member_id = match (existing, input.claim_member_id) {
        // Already in the group: joining again is a no-op.
        (Some((id, _)), _) => {
            return Ok(Json(Joined {
                group_id: group.id,
                member_id: id,
            }));
        }
        // Takes over a placeholder, keeping its expenses and balance.
        (None, Some(claim)) => {
            let placeholder: String = sqlx::query_scalar(
                "UPDATE members SET user_id = $3
                  WHERE id = $1 AND group_id = $2 AND user_id IS NULL AND left_at IS NULL
                 RETURNING display_name",
            )
            .bind(claim)
            .bind(group.id)
            .bind(user.id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                AppError::field("claim_member_id", "is not a placeholder in this group")
            })?;
            activity::record(
                &mut *tx,
                group.id,
                user.id,
                kind::MEMBER_JOINED,
                json!({ "member_id": claim, "name": placeholder, "claimed": true }),
                now,
            )
            .await?;
            claim
        }
        (None, None) => {
            members::ensure_room(&mut *tx, group.id).await?;
            let name = free_name(&mut tx, group.id, &user.name).await?;
            let id =
                members::insert_member(&mut *tx, group.id, Some(user.id), &name, "member", now)
                    .await?;
            activity::record(
                &mut *tx,
                group.id,
                user.id,
                kind::MEMBER_JOINED,
                json!({ "member_id": id, "name": name }),
                now,
            )
            .await?;
            id
        }
    };
    tx.commit().await?;

    Ok(Json(Joined {
        group_id: group.id,
        member_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::pick_free_name;

    #[test]
    fn keeps_the_name_when_it_is_free() {
        assert_eq!(pick_free_name("Sam", &["alex".into()]), "Sam");
    }

    #[test]
    fn numbers_clashing_names() {
        assert_eq!(pick_free_name("Sam", &["sam".into()]), "Sam 2");
        assert_eq!(
            pick_free_name("Sam", &["sam".into(), "sam 2".into(), "sam 3".into()]),
            "Sam 4"
        );
    }
}
