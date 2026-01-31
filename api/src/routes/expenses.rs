use std::collections::HashMap;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use super::members::{self, MemberView};
use crate::access;
use crate::activity::{self, kind};
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
        .route("/{expense_id}/history", get(history))
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
    let position: HashMap<Uuid, usize> =
        members.iter().enumerate().map(|(i, m)| (m.id, i)).collect();

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

    // Split in member order so leftover cents always go to the same people.
    let mut participants: Vec<Participant> = input
        .splits
        .iter()
        .map(|s| Participant {
            member_id: s.member_id,
            value: s.value,
        })
        .collect();
    participants.sort_by_key(|p| position[&p.member_id]);

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

async fn record_revision(
    conn: &mut PgConnection,
    expense: &ExpenseView,
    action: &str,
    changed_by: Uuid,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    let snapshot = serde_json::to_value(expense)
        .map_err(|e| AppError::Internal(format!("serialising snapshot: {e}")))?;
    sqlx::query(
        "INSERT INTO expense_revisions (id, expense_id, revision, action, snapshot, changed_by, changed_at)
         VALUES ($1, $2,
                 (SELECT COALESCE(max(revision), 0) + 1 FROM expense_revisions WHERE expense_id = $2),
                 $3, $4, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(expense.id)
    .bind(action)
    .bind(snapshot)
    .bind(changed_by)
    .bind(at)
    .execute(&mut *conn)
    .await?;
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
    record_revision(conn, &expense, "created", actor, at).await?;
    activity::record(
        &mut *conn,
        group_id,
        actor,
        kind::EXPENSE_ADDED,
        json!({
            "expense_id": id,
            "description": prepared.description,
            "amount_cents": prepared.amount,
            "paid_by": prepared.paid_by_name,
        }),
        at,
    )
    .await?;
    Ok(expense)
}

/// Names of the fields that differ between the stored expense and the edit.
fn changed_fields(before: &ExpenseView, after: &Prepared) -> Vec<&'static str> {
    let mut changed = Vec::new();
    if before.description != after.description {
        changed.push("description");
    }
    if before.amount_cents != after.amount {
        changed.push("amount");
    }
    if before.paid_by != after.paid_by {
        changed.push("paid_by");
    }
    if before.category != after.category {
        changed.push("category");
    }
    if before.spent_on != after.spent_on {
        changed.push("date");
    }
    if before.notes != after.notes {
        changed.push("notes");
    }
    let shares_after: Vec<ShareView> = after
        .shares
        .iter()
        .map(|(member_id, input, amount)| ShareView {
            member_id: *member_id,
            input: *input,
            amount_cents: *amount,
        })
        .collect();
    if before.split_method != after.method.as_str() || before.shares != shares_after {
        changed.push("split");
    }
    changed
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub category: Option<String>,
    pub q: Option<String>,
    /// Only expenses this member paid for or has a share in.
    pub member: Option<Uuid>,
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
    let category = query.category.filter(|c| !c.is_empty());
    let search = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(|q| format!("%{q}%"));

    const FILTER: &str = "e.group_id = $1 AND e.deleted_at IS NULL
           AND ($2::text IS NULL OR e.category = $2)
           AND ($3::text IS NULL OR e.description ILIKE $3 OR e.notes ILIKE $3)
           AND ($4::uuid IS NULL OR e.paid_by = $4
                OR EXISTS (SELECT 1 FROM expense_shares s
                            WHERE s.expense_id = e.id AND s.member_id = $4))";

    let total: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM expenses e WHERE {FILTER}"))
        .bind(group_id)
        .bind(&category)
        .bind(&search)
        .bind(query.member)
        .fetch_one(&state.pool)
        .await?;

    let mut items = sqlx::query_as::<_, ExpenseView>(&format!(
        "SELECT {EXPENSE_COLUMNS} FROM expenses e WHERE {FILTER}
          ORDER BY e.spent_on DESC, e.created_at DESC, e.id
          LIMIT $5 OFFSET $6"
    ))
    .bind(group_id)
    .bind(&category)
    .bind(&search)
    .bind(query.member)
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

    let changed = changed_fields(&before, &prepared);
    if changed.is_empty() {
        return Ok(Json(before));
    }

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
    record_revision(&mut tx, &after, "updated", auth.id, now).await?;
    activity::record(
        &mut *tx,
        group_id,
        auth.id,
        kind::EXPENSE_UPDATED,
        json!({
            "expense_id": expense_id,
            "description": after.description,
            "amount_cents": after.amount_cents,
            "changes": changed,
        }),
        now,
    )
    .await?;
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

    let expense = load_expense(&mut tx, group_id, expense_id).await?;
    sqlx::query("UPDATE expenses SET deleted_at = $2 WHERE id = $1")
        .bind(expense_id)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    record_revision(&mut tx, &expense, "deleted", auth.id, now).await?;
    activity::record(
        &mut *tx,
        group_id,
        auth.id,
        kind::EXPENSE_DELETED,
        json!({
            "expense_id": expense_id,
            "description": expense.description,
            "amount_cents": expense.amount_cents,
        }),
        now,
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Revision {
    pub revision: i32,
    pub action: String,
    pub snapshot: serde_json::Value,
    pub changed_by: Option<Uuid>,
    pub changed_by_name: Option<String>,
    pub changed_at: DateTime<Utc>,
}

async fn history(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((group_id, expense_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<Revision>>, AppError> {
    access::membership(&state.pool, group_id, auth.id).await?;
    let revisions = sqlx::query_as::<_, Revision>(
        "SELECT r.revision, r.action, r.snapshot, r.changed_by, u.name AS changed_by_name, r.changed_at
           FROM expense_revisions r
           JOIN expenses e ON e.id = r.expense_id
           LEFT JOIN users u ON u.id = r.changed_by
          WHERE r.expense_id = $1 AND e.group_id = $2
          ORDER BY r.revision",
    )
    .bind(expense_id)
    .bind(group_id)
    .fetch_all(&state.pool)
    .await?;
    if revisions.is_empty() {
        return Err(AppError::NotFound("expense"));
    }
    Ok(Json(revisions))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> ExpenseView {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        ExpenseView {
            id: Uuid::from_u128(9),
            group_id: Uuid::from_u128(8),
            description: "Groceries".into(),
            amount_cents: 1000,
            paid_by: a,
            split_method: "equal".into(),
            category: "groceries".into(),
            spent_on: NaiveDate::from_ymd_opt(2026, 1, 10).unwrap(),
            notes: None,
            created_by: Uuid::from_u128(7),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            shares: vec![
                ShareView {
                    member_id: a,
                    input: 1,
                    amount_cents: 500,
                },
                ShareView {
                    member_id: b,
                    input: 1,
                    amount_cents: 500,
                },
            ],
        }
    }

    fn prepared_from(view: &ExpenseView) -> Prepared {
        Prepared {
            description: view.description.clone(),
            amount: view.amount_cents,
            paid_by: view.paid_by,
            paid_by_name: "A".into(),
            method: view.split_method.parse().unwrap(),
            category: view.category.clone(),
            spent_on: view.spent_on,
            notes: view.notes.clone(),
            shares: view
                .shares
                .iter()
                .map(|s| (s.member_id, s.input, s.amount_cents))
                .collect(),
        }
    }

    #[test]
    fn unchanged_expense_has_no_changes() {
        let before = view();
        assert!(changed_fields(&before, &prepared_from(&before)).is_empty());
    }

    #[test]
    fn detects_changed_fields() {
        let before = view();
        let mut after = prepared_from(&before);
        after.description = "Weekly shop".into();
        after.notes = Some("Lidl".into());
        assert_eq!(
            changed_fields(&before, &after),
            vec!["description", "notes"]
        );
    }

    #[test]
    fn a_different_split_counts_as_a_change() {
        let before = view();
        let mut after = prepared_from(&before);
        after.method = SplitMethod::Exact;
        after.shares[0].1 = 300;
        after.shares[0].2 = 300;
        after.shares[1].1 = 700;
        after.shares[1].2 = 700;
        assert_eq!(changed_fields(&before, &after), vec!["split"]);
    }

    #[test]
    fn split_errors_point_at_the_right_field() {
        assert_eq!(
            split_error_field(&SplitError::NonPositiveTotal),
            "amount_cents"
        );
        assert_eq!(
            split_error_field(&SplitError::PercentMismatch { actual: 1 }),
            "splits"
        );
    }
}
