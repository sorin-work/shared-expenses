use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use super::members::{self, MemberView};
use crate::access;
use crate::auth::AuthUser;
use crate::domain::currency;
use crate::error::{AppError, is_unique_violation};
use crate::extract::{Json, Path};
use crate::state::AppState;
use crate::validate::Validator;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{group_id}", get(show))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct GroupSummary {
    pub id: Uuid,
    pub name: String,
    pub currency: String,
    pub member_count: i64,
    pub my_member_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct GroupRow {
    pub id: Uuid,
    pub name: String,
    pub currency: String,
    pub invite_code: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct GroupDetail {
    #[serde(flatten)]
    pub group: GroupRow,
    pub my_member_id: Uuid,
    pub my_role: String,
    pub members: Vec<MemberView>,
}

#[derive(Deserialize)]
pub struct CreateGroup {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub currency: String,
}

const INVITE_ALPHABET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
const INVITE_LENGTH: usize = 10;

/// A random, URL friendly invite code without look-alike characters.
pub fn new_invite_code() -> String {
    let mut rng = rand::rng();
    (0..INVITE_LENGTH)
        .map(|_| char::from(INVITE_ALPHABET[rng.random_range(0..INVITE_ALPHABET.len())]))
        .collect()
}

fn validate_currency(v: &mut Validator, value: &str) -> String {
    let code = value.trim().to_ascii_uppercase();
    if code.is_empty() {
        v.add("currency", "is required");
    } else if !currency::is_supported(&code) {
        v.add("currency", "is not a supported currency");
    }
    code
}

/// Creates a group with `owner_name` as its first member and owner.
pub async fn insert_group(
    conn: &mut PgConnection,
    owner_id: Uuid,
    owner_name: &str,
    name: &str,
    currency: &str,
    at: DateTime<Utc>,
) -> Result<(GroupRow, Uuid), AppError> {
    // Invite codes are random enough that a collision is practically
    // impossible, but retry a couple of times rather than fail.
    let mut attempts = 0;
    let group = loop {
        attempts += 1;
        let result = sqlx::query_as::<_, GroupRow>(
            "INSERT INTO groups (id, name, currency, invite_code, created_by, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, name, currency, invite_code, created_at",
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(currency)
        .bind(new_invite_code())
        .bind(owner_id)
        .bind(at)
        .fetch_one(&mut *conn)
        .await;
        match result {
            Ok(group) => break group,
            Err(err) if attempts < 3 && is_unique_violation(&err, "groups_invite_code_key") => {}
            Err(err) => return Err(err.into()),
        }
    };

    let member_id = members::insert_member(
        &mut *conn,
        group.id,
        Some(owner_id),
        owner_name,
        "owner",
        at,
    )
    .await?;
    Ok((group, member_id))
}

async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<GroupSummary>>, AppError> {
    let groups = sqlx::query_as::<_, GroupSummary>(
        "SELECT g.id, g.name, g.currency, g.created_at,
                m.id AS my_member_id,
                (SELECT count(*) FROM members mm
                  WHERE mm.group_id = g.id AND mm.left_at IS NULL) AS member_count
           FROM groups g
           JOIN members m ON m.group_id = g.id AND m.user_id = $1 AND m.left_at IS NULL
          ORDER BY g.created_at",
    )
    .bind(auth.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(groups))
}

async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateGroup>,
) -> Result<(StatusCode, Json<GroupDetail>), AppError> {
    let mut v = Validator::new();
    let name = v.text("name", &input.name, 80);
    let currency = validate_currency(&mut v, &input.currency);
    v.finish()?;

    let user = super::auth::load_user(&state.pool, auth.id).await?;
    let mut tx = state.pool.begin().await?;
    let (group, _) =
        insert_group(&mut tx, user.id, &user.name, &name, &currency, Utc::now()).await?;
    tx.commit().await?;

    let detail = load_detail(&state.pool, group.id, auth.id).await?;
    Ok((StatusCode::CREATED, Json(detail)))
}

async fn load_detail(
    pool: &sqlx::PgPool,
    group_id: Uuid,
    user_id: Uuid,
) -> Result<GroupDetail, AppError> {
    let membership = access::membership(pool, group_id, user_id).await?;
    let group = sqlx::query_as::<_, GroupRow>(
        "SELECT id, name, currency, invite_code, created_at FROM groups WHERE id = $1",
    )
    .bind(group_id)
    .fetch_one(pool)
    .await?;
    let members = members::list_members(pool, group_id).await?;
    Ok(GroupDetail {
        group,
        my_member_id: membership.member_id,
        my_role: membership.role,
        members,
    })
}

async fn show(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<Uuid>,
) -> Result<Json<GroupDetail>, AppError> {
    Ok(Json(load_detail(&state.pool, group_id, auth.id).await?))
}
