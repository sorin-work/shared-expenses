//! CSV export of a group's expenses and settlements, one column per member
//! with the effect of each row on that member's balance.

use std::collections::HashMap;

use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use chrono::NaiveDate;
use uuid::Uuid;

use super::members;
use crate::access;
use crate::auth::AuthUser;
use crate::csv::CsvWriter;
use crate::domain::{currency, money};
use crate::error::AppError;
use crate::extract::Path;
use crate::state::AppState;

struct Row {
    date: NaiveDate,
    kind: &'static str,
    description: String,
    category: String,
    paid_by: Uuid,
    amount: i64,
    /// member -> change of their balance
    effect: HashMap<Uuid, i64>,
}

/// A file name like `flat-4b-expenses.csv`.
fn file_name(group_name: &str) -> String {
    let mut slug = String::new();
    for c in group_name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "expenses.csv".to_string()
    } else {
        format!("{slug}-expenses.csv")
    }
}

pub async fn csv(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    access::membership(&state.pool, group_id, auth.id).await?;
    let (name, code): (String, String) =
        sqlx::query_as("SELECT name, currency FROM groups WHERE id = $1")
            .bind(group_id)
            .fetch_one(&state.pool)
            .await?;
    let decimals = currency::minor_units(code.trim()).unwrap_or(2);
    let members = members::list_members(&state.pool, group_id).await?;

    let expenses: Vec<(Uuid, NaiveDate, String, String, Uuid, i64)> = sqlx::query_as(
        "SELECT id, spent_on, description, category, paid_by, amount_cents FROM expenses
          WHERE group_id = $1 AND deleted_at IS NULL",
    )
    .bind(group_id)
    .fetch_all(&state.pool)
    .await?;
    let shares: Vec<(Uuid, Uuid, i64)> = sqlx::query_as(
        "SELECT s.expense_id, s.member_id, s.amount_cents FROM expense_shares s
           JOIN expenses e ON e.id = s.expense_id
          WHERE e.group_id = $1 AND e.deleted_at IS NULL",
    )
    .bind(group_id)
    .fetch_all(&state.pool)
    .await?;
    let settlements: Vec<(NaiveDate, Option<String>, Uuid, Uuid, i64)> = sqlx::query_as(
        "SELECT settled_on, note, from_member, to_member, amount_cents FROM settlements
          WHERE group_id = $1 AND deleted_at IS NULL",
    )
    .bind(group_id)
    .fetch_all(&state.pool)
    .await?;

    let mut shares_by_expense: HashMap<Uuid, Vec<(Uuid, i64)>> = HashMap::new();
    for (expense_id, member_id, amount) in shares {
        shares_by_expense
            .entry(expense_id)
            .or_default()
            .push((member_id, amount));
    }

    let mut rows: Vec<Row> = Vec::new();
    for (id, date, description, category, paid_by, amount) in expenses {
        let mut effect: HashMap<Uuid, i64> = HashMap::new();
        *effect.entry(paid_by).or_default() += amount;
        for (member_id, share) in shares_by_expense.remove(&id).unwrap_or_default() {
            *effect.entry(member_id).or_default() -= share;
        }
        rows.push(Row {
            date,
            kind: "expense",
            description,
            category,
            paid_by,
            amount,
            effect,
        });
    }
    for (date, note, from, to, amount) in settlements {
        rows.push(Row {
            date,
            kind: "settlement",
            description: note.unwrap_or_else(|| "Settlement".to_string()),
            category: String::new(),
            paid_by: from,
            amount,
            effect: HashMap::from([(from, amount), (to, -amount)]),
        });
    }
    rows.sort_by(|a, b| a.date.cmp(&b.date).then(a.kind.cmp(b.kind)));

    let name_of = |id: Uuid| {
        members
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.display_name.clone())
            .unwrap_or_default()
    };

    let mut out = CsvWriter::new();
    let mut header_row = vec![
        "Date".to_string(),
        "Type".to_string(),
        "Description".to_string(),
        "Category".to_string(),
        "Paid by".to_string(),
        format!("Amount ({})", code.trim()),
    ];
    header_row.extend(members.iter().map(|m| m.display_name.clone()));
    out.row(&header_row);

    let mut totals: HashMap<Uuid, i64> = HashMap::new();
    for row in &rows {
        let mut fields = vec![
            row.date.to_string(),
            row.kind.to_string(),
            row.description.clone(),
            row.category.clone(),
            name_of(row.paid_by),
            money::format_minor(row.amount, decimals),
        ];
        for member in &members {
            let effect = row.effect.get(&member.id).copied().unwrap_or(0);
            *totals.entry(member.id).or_default() += effect;
            fields.push(money::format_minor(effect, decimals));
        }
        out.row(&fields);
    }

    let mut total_row = vec![
        String::new(),
        "balance".to_string(),
        "Balance".to_string(),
        String::new(),
        String::new(),
        String::new(),
    ];
    total_row.extend(
        members
            .iter()
            .map(|m| money::format_minor(totals.get(&m.id).copied().unwrap_or(0), decimals)),
    );
    out.row(&total_row);

    let disposition = format!("attachment; filename=\"{}\"", file_name(&name));
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        out.finish(),
    ))
}

#[cfg(test)]
mod tests {
    use super::file_name;

    #[test]
    fn builds_file_names_from_group_names() {
        assert_eq!(file_name("Flat 4B"), "flat-4b-expenses.csv");
        assert_eq!(file_name("  Lisbon '26!! "), "lisbon-26-expenses.csv");
        assert_eq!(file_name("Ünïcode"), "n-code-expenses.csv");
        assert_eq!(file_name("***"), "expenses.csv");
    }
}
