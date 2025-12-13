use std::collections::HashMap;

use axum::extract::State;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::members::{self, MemberView};
use crate::access;
use crate::auth::AuthUser;
use crate::domain::balance::{self, ExpenseEntry, MemberBalance, SettlementEntry};
use crate::domain::split::Share;
use crate::error::AppError;
use crate::extract::{Json, Path};
use crate::state::AppState;

#[derive(Serialize)]
pub struct BalanceRow {
    pub member_id: Uuid,
    pub display_name: String,
    pub left: bool,
    #[serde(flatten)]
    pub balance: MemberBalance,
}

#[derive(Serialize)]
pub struct Balances {
    pub currency: String,
    pub balances: Vec<BalanceRow>,
}

/// Loads every live expense and settlement of a group and computes the
/// balances of its members (in member order).
pub async fn compute_for_group(
    pool: &PgPool,
    group_id: Uuid,
) -> Result<(Vec<MemberView>, Vec<MemberBalance>), AppError> {
    let members = members::list_members(pool, group_id).await?;

    let rows: Vec<(Uuid, Uuid, i64, Uuid, i64)> = sqlx::query_as(
        "SELECT e.id, e.paid_by, e.amount_cents, s.member_id, s.amount_cents
           FROM expenses e
           JOIN expense_shares s ON s.expense_id = e.id
          WHERE e.group_id = $1 AND e.deleted_at IS NULL",
    )
    .bind(group_id)
    .fetch_all(pool)
    .await?;

    let mut expenses: HashMap<Uuid, ExpenseEntry> = HashMap::new();
    for (expense_id, paid_by, amount, member_id, share) in rows {
        expenses
            .entry(expense_id)
            .or_insert_with(|| ExpenseEntry {
                paid_by,
                amount,
                shares: Vec::new(),
            })
            .shares
            .push(Share {
                member_id,
                amount: share,
            });
    }
    let expenses: Vec<ExpenseEntry> = expenses.into_values().collect();

    let settlements: Vec<SettlementEntry> = sqlx::query_as::<_, (Uuid, Uuid, i64)>(
        "SELECT from_member, to_member, amount_cents FROM settlements
          WHERE group_id = $1 AND deleted_at IS NULL",
    )
    .bind(group_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(from, to, amount)| SettlementEntry { from, to, amount })
    .collect();

    let ids: Vec<Uuid> = members.iter().map(|m| m.id).collect();
    let balances = balance::compute(&ids, &expenses, &settlements);
    Ok((members, balances))
}

pub async fn show(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<Uuid>,
) -> Result<Json<Balances>, AppError> {
    access::membership(&state.pool, group_id, auth.id).await?;
    let currency: String = sqlx::query_scalar("SELECT currency FROM groups WHERE id = $1")
        .bind(group_id)
        .fetch_one(&state.pool)
        .await?;
    let (members, balances) = compute_for_group(&state.pool, group_id).await?;

    let balances = balances
        .into_iter()
        .filter_map(|balance| {
            let member = members.iter().find(|m| m.id == balance.member_id)?;
            // Members who left with a zero balance are just noise here.
            if member.left_at.is_some() && balance.net == 0 {
                return None;
            }
            Some(BalanceRow {
                member_id: member.id,
                display_name: member.display_name.clone(),
                left: member.left_at.is_some(),
                balance,
            })
        })
        .collect();

    Ok(Json(Balances { currency, balances }))
}
