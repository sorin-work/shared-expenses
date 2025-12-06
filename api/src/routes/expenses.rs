use std::collections::HashMap;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use super::members::{self, MemberView};
use crate::access;
use crate::auth::AuthUser;
use crate::domain::category;
use crate::domain::split::{self, Participant, SplitError, SplitMethod};
use crate::error::AppError;
use crate::extract::{Json, Path, Query};
use crate::state::AppState;
use crate::validate::Validator;

/// 100 million in major units is plenty for shared bills.
pub const MAX_AMOUNT: i64 = 10_000_000_000;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{expense_id}", get(show).put(update).delete(remove))
}

#[derive(Debug, Clone, Deserialize)]
pub struct SplitInput {
    pub member_id: Uuid,
    /// Exact amount, basis points or shares depending on the split method.
    /// Not needed for equal splits.
    #[serde(default)]
    pub value: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExpenseInput {
    #[serde(default)]
    pub description: String,
    pub amount_cents: i64,
    pub paid_by: Uuid,
    #[serde(default)]
    pub split_method: String,
    #[serde(default)]
    pub category: Option<String>,
    pub spent_on: NaiveDate,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub splits: Vec<SplitInput>,
}

/// A validated expense, ready to be written.
#[derive(Debug, Clone)]
pub struct Prepared {
    pub description: String,
    pub amount: i64,
    pub paid_by: Uuid,
    pub paid_by_name: String,
    pub method: SplitMethod,
    pub category: String,
    pub spent_on: NaiveDate,
    pub notes: Option<String>,
    /// (member, value as entered, share in minor units)
    pub shares: Vec<(Uuid, i64, i64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::FromRow)]
pub struct ShareView {
    pub member_id: Uuid,
    pub input: i64,
    pub amount_cents: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ExpenseView {
    pub id: Uuid,
    pub group_id: Uuid,
    pub description: String,
    pub amount_cents: i64,
    pub paid_by: Uuid,
    pub split_method: String,
    pub category: String,
    pub spent_on: NaiveDate,
    pub notes: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[sqlx(skip)]
    pub shares: Vec<ShareView>,
}

const EXPENSE_COLUMNS: &str = "id, group_id, description, amount_cents, paid_by, split_method, \
     category, spent_on, notes, created_by, created_at, updated_at";

fn split_error_field(err: &SplitError) -> &'static str {
    match err {
        SplitError::NonPositiveTotal => "amount_cents",
        _ => "splits",
    }
}

/// Validates an expense against the group's members. `also_allowed` lists
/// members who left the group but may still appear, because they were part
/// of the expense being edited.
pub async fn prepare(
    db: impl PgExecutor<'_>,
    group_id: Uuid,
    input: &ExpenseInput,
    also_allowed: &[Uuid],
    today: NaiveDate,
) -> Result<Prepared, AppError> {
    let mut v = Validator::new();
    let description = v.text("description", &input.description, 120);
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

    let method = input.split_method.parse::<SplitMethod>().ok();
    v.check(
        method.is_some(),
        "split_method",
        "must be equal, exact, percent or shares",
    );

    let category = input
        .category
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .unwrap_or(category::DEFAULT)
        .to_string();
    v.check(
        category::is_valid(&category),
        "category",
        "is not a known category",
    );

    let notes = v.optional_text("notes", input.notes.as_deref(), 1000);

    v.check(
        input.spent_on <= today,
        "spent_on",
        "can't be in the future",
    );
    v.check(
        input.spent_on >= NaiveDate::from_ymd_opt(2000, 1, 1).expect("valid date"),
        "spent_on",
        "is too far in the past",
    );

    let members: Vec<MemberView> = members::list_members(db, group_id).await?;
    let usable = |id: &Uuid| {
        members
            .iter()
            .any(|m| m.id == *id && (m.left_at.is_none() || also_allowed.contains(id)))
    };

    v.check(
        usable(&input.paid_by),
        "paid_by",
        "is not a member of this group",
    );
    if input.splits.is_empty() {
        v.add("splits", "pick at least one person to split with");
    } else if !input.splits.iter().all(|s| usable(&s.member_id)) {
        v.add(
            "splits",
            "includes someone who is not a member of this group",
        );
    }
    v.finish()?;
    let method = method.expect("checked above");

    let participants: Vec<Participant> = input
        .splits
        .iter()
        .map(|s| Participant {
            member_id: s.member_id,
            value: s.value,
        })
        .collect();

    let shares = split::split(input.amount_cents, method, &participants)
        .map_err(|err| AppError::field(split_error_field(&err), err.to_string()))?;

    let paid_by_name = members
        .iter()
        .find(|m| m.id == input.paid_by)
        .map(|m| m.display_name.clone())
        .unwrap_or_default();

    Ok(Prepared {
        description,
        amount: input.amount_cents,
        paid_by: input.paid_by,
        paid_by_name,
        method,
        category,
        spent_on: input.spent_on,
        notes,
        shares: participants
            .iter()
            .zip(shares)
            .map(|(p, s)| {
                // Equal splits have no meaningful input; store 1 share each.
                let input = if method == SplitMethod::Equal {
                    1
                } else {
                    p.value
                };
                (p.member_id, input, s.amount)
            })
            .collect(),
    })
}

async fn load_shares(
    db: impl PgExecutor<'_>,
    expense_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<ShareView>>, AppError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        expense_id: Uuid,
        #[sqlx(flatten)]
        share: ShareView,
    }

    let rows = sqlx::query_as::<_, Row>(
        "SELECT s.expense_id, s.member_id, s.input, s.amount_cents
           FROM expense_shares s
           JOIN members m ON m.id = s.member_id
          WHERE s.expense_id = ANY($1)
          ORDER BY m.joined_at, m.id",
    )
    .bind(expense_ids)
    .fetch_all(db)
    .await?;

    let mut by_expense: HashMap<Uuid, Vec<ShareView>> = HashMap::new();
    for row in rows {
        by_expense
            .entry(row.expense_id)
            .or_default()
            .push(row.share);
    }
    Ok(by_expense)
}

pub async fn load_expense(
    conn: &mut PgConnection,
    group_id: Uuid,
    expense_id: Uuid,
) -> Result<ExpenseView, AppError> {
    let mut expense = sqlx::query_as::<_, ExpenseView>(&format!(
        "SELECT {EXPENSE_COLUMNS} FROM expenses
          WHERE id = $1 AND group_id = $2 AND deleted_at IS NULL"
    ))
    .bind(expense_id)
    .bind(group_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(AppError::NotFound("expense"))?;
    expense.shares = load_shares(&mut *conn, &[expense.id])
        .await?
        .remove(&expense.id)
        .unwrap_or_default();
    Ok(expense)
}

async fn insert_shares(
    conn: &mut PgConnection,
    expense_id: Uuid,
    prepared: &Prepared,
) -> Result<(), AppError> {
    for (member_id, input, amount) in &prepared.shares {
        sqlx::query(
            "INSERT INTO expense_shares (expense_id, member_id, input, amount_cents)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(expense_id)
        .bind(member_id)
        .bind(input)
        .bind(amount)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Writes a new expense with its shares, first revision and feed entry.
pub async fn insert_expense(
    conn: &mut PgConnection,
    group_id: Uuid,
    actor: Uuid,
    prepared: &Prepared,
    at: DateTime<Utc>,
) -> Result<ExpenseView, AppError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO expenses (id, group_id, description, amount_cents, paid_by, split_method,
                               category, spent_on, notes, created_by, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)",
    )
    .bind(id)
    .bind(group_id)
    .bind(&prepared.description)
    .bind(prepared.amount)
    .bind(prepared.paid_by)
    .bind(prepared.method.as_str())
    .bind(&prepared.category)
    .bind(prepared.spent_on)
    .bind(&prepared.notes)
    .bind(actor)
    .bind(at)
    .execute(&mut *conn)
    .await?;
    insert_shares(conn, id, prepared).await?;

    let expense = load_expense(conn, group_id, id).await?;
    Ok(expense)
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct ExpensePage {
    pub items: Vec<ExpenseView>,
    pub total: i64,
}

async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<Uuid>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ExpensePage>, AppError> {
    access::membership(&state.pool, group_id, auth.id).await?;

    let limit = query.limit.unwrap_or(50).max(1);
    let offset = query.offset.unwrap_or(0).max(0);
    let total: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM expenses WHERE group_id = $1 AND deleted_at IS NULL",
    )
    .bind(group_id)
    .fetch_one(&state.pool)
    .await?;

    let mut items = sqlx::query_as::<_, ExpenseView>(&format!(
        "SELECT {EXPENSE_COLUMNS} FROM expenses
          WHERE group_id = $1 AND deleted_at IS NULL
          ORDER BY spent_on DESC, created_at DESC, id
          LIMIT $2 OFFSET $3"
    ))
    .bind(group_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let ids: Vec<Uuid> = items.iter().map(|e| e.id).collect();
    let mut shares = load_shares(&state.pool, &ids).await?;
    for item in &mut items {
        item.shares = shares.remove(&item.id).unwrap_or_default();
    }
    Ok(Json(ExpensePage { items, total }))
}

async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<Uuid>,
    Json(input): Json<ExpenseInput>,
) -> Result<(StatusCode, Json<ExpenseView>), AppError> {
    access::membership(&state.pool, group_id, auth.id).await?;
    let now = Utc::now();

    let mut tx = state.pool.begin().await?;
    let prepared = prepare(&mut *tx, group_id, &input, &[], now.date_naive()).await?;
    let expense = insert_expense(&mut tx, group_id, auth.id, &prepared, now).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(expense)))
}

async fn show(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((group_id, expense_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ExpenseView>, AppError> {
    access::membership(&state.pool, group_id, auth.id).await?;
    let mut conn = state.pool.acquire().await?;
    Ok(Json(load_expense(&mut conn, group_id, expense_id).await?))
}

async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((group_id, expense_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<ExpenseInput>,
) -> Result<Json<ExpenseView>, AppError> {
    access::membership(&state.pool, group_id, auth.id).await?;
    let now = Utc::now();
    let mut tx = state.pool.begin().await?;

    // Lock the row so two edits at the same time get consecutive revisions.
    sqlx::query("SELECT 1 FROM expenses WHERE id = $1 AND group_id = $2 FOR UPDATE")
        .bind(expense_id)
        .bind(group_id)
        .execute(&mut *tx)
        .await?;
    let before = load_expense(&mut tx, group_id, expense_id).await?;

    let mut involved: Vec<Uuid> = before.shares.iter().map(|s| s.member_id).collect();
    involved.push(before.paid_by);
    let prepared = prepare(&mut *tx, group_id, &input, &involved, now.date_naive()).await?;

    sqlx::query(
        "UPDATE expenses
            SET description = $2, amount_cents = $3, paid_by = $4, split_method = $5,
                category = $6, spent_on = $7, notes = $8, updated_at = $9
          WHERE id = $1",
    )
    .bind(expense_id)
    .bind(&prepared.description)
    .bind(prepared.amount)
    .bind(prepared.paid_by)
    .bind(prepared.method.as_str())
    .bind(&prepared.category)
    .bind(prepared.spent_on)
    .bind(&prepared.notes)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM expense_shares WHERE expense_id = $1")
        .bind(expense_id)
        .execute(&mut *tx)
        .await?;
    insert_shares(&mut tx, expense_id, &prepared).await?;

    let after = load_expense(&mut tx, group_id, expense_id).await?;
    tx.commit().await?;
    Ok(Json(after))
}

async fn remove(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((group_id, expense_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    access::membership(&state.pool, group_id, auth.id).await?;
    let now = Utc::now();
    let mut tx = state.pool.begin().await?;

    load_expense(&mut tx, group_id, expense_id).await?;
    sqlx::query("UPDATE expenses SET deleted_at = $2 WHERE id = $1")
        .bind(expense_id)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
