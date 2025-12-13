//! Recording money paid back between members.

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{delete, get};
use chrono::{DateTime, Days, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use super::expenses::MAX_AMOUNT;
use super::members;
use crate::access;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::extract::{Json, Path};
use crate::state::AppState;
use crate::validate::Validator;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{settlement_id}", delete(remove))
}

#[derive(Debug, Clone, Deserialize)]
pub struct SettlementInput {
    pub from_member: Uuid,
    pub to_member: Uuid,
    pub amount_cents: i64,
    pub settled_on: NaiveDate,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SettlementView {
    pub id: Uuid,
    pub from_member: Uuid,
    pub to_member: Uuid,
    pub amount_cents: i64,
    pub settled_on: NaiveDate,
    pub note: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

const COLUMNS: &str =
    "id, from_member, to_member, amount_cents, settled_on, note, created_by, created_at";

/// Validates and stores a settlement, and adds it to the feed.
pub async fn insert_settlement(
    conn: &mut PgConnection,
    group_id: Uuid,
    actor: Uuid,
    input: &SettlementInput,
    at: DateTime<Utc>,
) -> Result<SettlementView, AppError> {
    let mut v = Validator::new();
    v.check(
        input.amount_cents > 0,
        "amount_cents",
        "must be greater than zero",
    );
    v.check(
        input.amount_cents <= MAX_AMOUNT,
        "amount_cents",
        "is too large",
    );
    v.check(
        input.from_member != input.to_member,
        "to_member",
        "must be someone else",
    );
    let latest = at.date_naive().checked_add_days(Days::new(1));
    v.check(
        latest.is_none_or(|latest| input.settled_on <= latest),
        "settled_on",
        "can't be in the future",
    );
    let note = v.optional_text("note", input.note.as_deref(), 200);

    let members = members::list_members(&mut *conn, group_id).await?;
    let name_of = |id: Uuid| {
        members
            .iter()
            .find(|m| m.id == id && m.left_at.is_none())
            .map(|m| m.display_name.clone())
    };
    let from_name = name_of(input.from_member);
    let to_name = name_of(input.to_member);
    v.check(
        from_name.is_some(),
        "from_member",
        "is not a member of this group",
    );
    v.check(
        to_name.is_some(),
        "to_member",
        "is not a member of this group",
    );
    v.finish()?;

    let settlement = sqlx::query_as::<_, SettlementView>(&format!(
        "INSERT INTO settlements (id, group_id, from_member, to_member, amount_cents, settled_on,
                                  note, created_by, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         RETURNING {COLUMNS}"
    ))
    .bind(Uuid::new_v4())
    .bind(group_id)
    .bind(input.from_member)
    .bind(input.to_member)
    .bind(input.amount_cents)
    .bind(input.settled_on)
    .bind(&note)
    .bind(actor)
    .bind(at)
    .fetch_one(&mut *conn)
    .await?;

    Ok(settlement)
}

async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<Uuid>,
) -> Result<Json<Vec<SettlementView>>, AppError> {
    access::membership(&state.pool, group_id, auth.id).await?;
    let settlements = sqlx::query_as::<_, SettlementView>(&format!(
        "SELECT {COLUMNS} FROM settlements
          WHERE group_id = $1 AND deleted_at IS NULL
          ORDER BY settled_on DESC, created_at DESC"
    ))
    .bind(group_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(settlements))
}

async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<Uuid>,
    Json(input): Json<SettlementInput>,
) -> Result<(StatusCode, Json<SettlementView>), AppError> {
    access::membership(&state.pool, group_id, auth.id).await?;
    let mut tx = state.pool.begin().await?;
    let settlement = insert_settlement(&mut tx, group_id, auth.id, &input, Utc::now()).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(settlement)))
}

async fn remove(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((group_id, settlement_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    access::membership(&state.pool, group_id, auth.id).await?;
    let mut tx = state.pool.begin().await?;

    let deleted = sqlx::query(
        "UPDATE settlements SET deleted_at = now()
          WHERE id = $1 AND group_id = $2 AND deleted_at IS NULL",
    )
    .bind(settlement_id)
    .bind(group_id)
    .execute(&mut *tx)
    .await?;
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("settlement"));
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
