use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{patch, post};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::access;
use crate::activity::{self, kind};
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::extract::{Json, Path};
use crate::state::AppState;
use crate::validate::Validator;

pub const MAX_MEMBERS: i64 = 50;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(add_placeholder))
        .route("/{member_id}", patch(rename).delete(remove))
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MemberView {
    pub id: Uuid,
    pub display_name: String,
    pub role: String,
    /// False for placeholder people without an account.
    pub has_account: bool,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
}

/// All members including the ones who left, in joining order. This order is
/// also used to hand out leftover cents when splitting.
pub async fn list_members(
    db: impl PgExecutor<'_>,
    group_id: Uuid,
) -> Result<Vec<MemberView>, AppError> {
    Ok(sqlx::query_as::<_, MemberView>(
        "SELECT id, display_name, role, user_id IS NOT NULL AS has_account, joined_at, left_at
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

/// The member's net balance: what they paid minus their shares, adjusted
/// for settlements.
pub async fn member_balance(db: impl PgExecutor<'_>, member_id: Uuid) -> Result<i64, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT (
              COALESCE((SELECT sum(amount_cents) FROM expenses
                         WHERE paid_by = $1 AND deleted_at IS NULL), 0)
            - COALESCE((SELECT sum(s.amount_cents) FROM expense_shares s
                          JOIN expenses e ON e.id = s.expense_id
                         WHERE s.member_id = $1 AND e.deleted_at IS NULL), 0)
            + COALESCE((SELECT sum(amount_cents) FROM settlements
                         WHERE from_member = $1 AND deleted_at IS NULL), 0)
            - COALESCE((SELECT sum(amount_cents) FROM settlements
                         WHERE to_member = $1 AND deleted_at IS NULL), 0)
         )::bigint",
    )
    .bind(member_id)
    .fetch_one(db)
    .await?)
}

/// Fails if another active member already uses this name.
pub async fn ensure_name_available(
    db: impl PgExecutor<'_>,
    group_id: Uuid,
    name: &str,
    except: Option<Uuid>,
) -> Result<(), AppError> {
    let taken: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM members
                         WHERE group_id = $1 AND left_at IS NULL
                           AND lower(display_name) = lower($2)
                           AND id IS DISTINCT FROM $3)",
    )
    .bind(group_id)
    .bind(name)
    .bind(except)
    .fetch_one(db)
    .await?;
    if taken {
        Err(AppError::field(
            "display_name",
            "someone in the group already has this name",
        ))
    } else {
        Ok(())
    }
}

pub async fn ensure_room(db: impl PgExecutor<'_>, group_id: Uuid) -> Result<(), AppError> {
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM members WHERE group_id = $1 AND left_at IS NULL")
            .bind(group_id)
            .fetch_one(db)
            .await?;
    if count >= MAX_MEMBERS {
        Err(AppError::Conflict(format!(
            "a group can have at most {MAX_MEMBERS} members"
        )))
    } else {
        Ok(())
    }
}

async fn find_member(
    pool: &PgPool,
    group_id: Uuid,
    member_id: Uuid,
) -> Result<MemberRow, AppError> {
    sqlx::query_as::<_, MemberRow>(
        "SELECT id, user_id, display_name, role FROM members
          WHERE id = $1 AND group_id = $2 AND left_at IS NULL",
    )
    .bind(member_id)
    .bind(group_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound("member"))
}

#[derive(sqlx::FromRow)]
struct MemberRow {
    id: Uuid,
    user_id: Option<Uuid>,
    display_name: String,
    role: String,
}

#[derive(Deserialize)]
pub struct MemberInput {
    #[serde(default)]
    pub display_name: String,
}

async fn add_placeholder(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<Uuid>,
    Json(input): Json<MemberInput>,
) -> Result<(StatusCode, Json<MemberView>), AppError> {
    access::membership(&state.pool, group_id, auth.id).await?;
    let mut v = Validator::new();
    let name = v.text("display_name", &input.display_name, 60);
    v.finish()?;

    let mut tx = state.pool.begin().await?;
    ensure_room(&mut *tx, group_id).await?;
    ensure_name_available(&mut *tx, group_id, &name, None).await?;
    let now = Utc::now();
    let id = insert_member(&mut *tx, group_id, None, &name, "member", now).await?;
    activity::record(
        &mut *tx,
        group_id,
        auth.id,
        kind::MEMBER_ADDED,
        json!({ "member_id": id, "name": name }),
        now,
    )
    .await?;
    tx.commit().await?;

    let member = list_members(&state.pool, group_id)
        .await?
        .into_iter()
        .find(|m| m.id == id)
        .ok_or(AppError::NotFound("member"))?;
    Ok((StatusCode::CREATED, Json(member)))
}

async fn rename(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((group_id, member_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<MemberInput>,
) -> Result<Json<MemberView>, AppError> {
    let me = access::membership(&state.pool, group_id, auth.id).await?;
    let target = find_member(&state.pool, group_id, member_id).await?;
    // Placeholders can be renamed by anyone in the group, people with an
    // account only by themselves or the owner.
    let allowed = me.is_owner() || target.user_id.is_none() || target.id == me.member_id;
    if !allowed {
        return Err(AppError::Forbidden("you can't rename this member"));
    }

    let mut v = Validator::new();
    let name = v.text("display_name", &input.display_name, 60);
    v.finish()?;

    if name != target.display_name {
        let mut tx = state.pool.begin().await?;
        ensure_name_available(&mut *tx, group_id, &name, Some(target.id)).await?;
        sqlx::query("UPDATE members SET display_name = $2 WHERE id = $1")
            .bind(target.id)
            .bind(&name)
            .execute(&mut *tx)
            .await?;
        activity::record(
            &mut *tx,
            group_id,
            auth.id,
            kind::MEMBER_RENAMED,
            json!({ "member_id": target.id, "from": target.display_name, "to": name }),
            Utc::now(),
        )
        .await?;
        tx.commit().await?;
    }

    let member = list_members(&state.pool, group_id)
        .await?
        .into_iter()
        .find(|m| m.id == target.id)
        .ok_or(AppError::NotFound("member"))?;
    Ok(Json(member))
}

async fn remove(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((group_id, member_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let me = access::membership(&state.pool, group_id, auth.id).await?;
    let target = find_member(&state.pool, group_id, member_id).await?;
    let leaving = target.id == me.member_id;

    if target.role == "owner" {
        return Err(AppError::Conflict(
            "the owner can't leave the group; delete the group instead".into(),
        ));
    }
    if !leaving && !me.is_owner() {
        return Err(AppError::Forbidden(
            "only the group owner can remove members",
        ));
    }

    let mut tx = state.pool.begin().await?;
    let referenced: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM expenses WHERE paid_by = $1)
             OR EXISTS (SELECT 1 FROM expense_shares WHERE member_id = $1)
             OR EXISTS (SELECT 1 FROM settlements WHERE from_member = $1 OR to_member = $1)",
    )
    .bind(target.id)
    .fetch_one(&mut *tx)
    .await?;

    if referenced {
        if member_balance(&mut *tx, target.id).await? != 0 {
            return Err(AppError::Conflict(format!(
                "{} still has an open balance; settle up first",
                target.display_name
            )));
        }
        sqlx::query("UPDATE members SET left_at = now() WHERE id = $1")
            .bind(target.id)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query("DELETE FROM members WHERE id = $1")
            .bind(target.id)
            .execute(&mut *tx)
            .await?;
    }

    activity::record(
        &mut *tx,
        group_id,
        auth.id,
        kind::MEMBER_REMOVED,
        json!({ "member_id": target.id, "name": target.display_name, "left": leaving }),
        Utc::now(),
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
