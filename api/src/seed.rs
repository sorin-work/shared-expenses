//! Demo data: `SEED_DEMO=true` creates demo@example.com / demo-password
//! with a flat share, a recent trip and a couple's household budget. Dates
//! are relative to today so the data always looks current.

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::auth::password;
use crate::error::AppError;
use crate::routes::auth::{User, create_user};
use crate::routes::expenses::{self, ExpenseInput, SplitInput};
use crate::routes::groups::insert_group;
use crate::routes::members::insert_member;
use crate::routes::settlements::{SettlementInput, insert_settlement};

pub const DEMO_EMAIL: &str = "demo@example.com";
pub const DEMO_PASSWORD: &str = "demo-password";

/// Creates the demo account and its data unless it already exists.
pub async fn run(pool: &PgPool) -> Result<(), AppError> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE lower(email) = $1)")
            .bind(DEMO_EMAIL)
            .fetch_one(pool)
            .await?;
    if exists {
        tracing::info!("demo data already present");
        return Ok(());
    }

    let hash = password::hash_async(DEMO_PASSWORD.to_string()).await?;
    let demo = create_user(pool, DEMO_EMAIL, "Alex Morgan", &hash).await?;
    let maya = create_user(pool, "maya@example.com", "Maya", &hash).await?;
    let jonas = create_user(pool, "jonas@example.com", "Jonas", &hash).await?;

    let today = Utc::now().date_naive();
    let mut tx = pool.begin().await?;
    flat_share(&mut tx, today, &demo, &maya, &jonas).await?;
    tx.commit().await?;

    tracing::info!(email = DEMO_EMAIL, "demo data created");
    Ok(())
}

fn day(today: NaiveDate, days_ago: i64) -> NaiveDate {
    today - Duration::days(days_ago)
}

/// An evening timestamp on the given day, so the feed looks like people
/// entered things after work.
fn evening(date: NaiveDate, minute: u32) -> DateTime<Utc> {
    let time = NaiveTime::from_hms_opt(19 + minute / 60 % 3, minute % 60, 0).expect("valid time");
    date.and_time(time).and_utc()
}

struct Ctx<'a> {
    conn: &'a mut PgConnection,
    group_id: Uuid,
    today: NaiveDate,
    counter: u32,
}

#[derive(Clone, Copy)]
enum Split<'a> {
    Equal(&'a [Uuid]),
    Exact(&'a [(Uuid, i64)]),
}

impl Ctx<'_> {
    fn next_minute(&mut self) -> u32 {
        self.counter += 7;
        self.counter
    }

    #[allow(clippy::too_many_arguments)]
    async fn expense(
        &mut self,
        actor: Uuid,
        days_ago: i64,
        description: &str,
        category: &str,
        amount_cents: i64,
        paid_by: Uuid,
        split: Split<'_>,
    ) -> Result<Uuid, AppError> {
        let input = expense_input(
            description,
            category,
            amount_cents,
            paid_by,
            day(self.today, days_ago),
            split,
        );
        let at = evening(input.spent_on, self.next_minute());
        let prepared =
            expenses::prepare(&mut *self.conn, self.group_id, &input, &[], self.today).await?;
        let expense =
            expenses::insert_expense(self.conn, self.group_id, actor, &prepared, at).await?;
        Ok(expense.id)
    }

    async fn settle(
        &mut self,
        actor: Uuid,
        days_ago: i64,
        from: Uuid,
        to: Uuid,
        amount_cents: i64,
        note: Option<&str>,
    ) -> Result<(), AppError> {
        let settled_on = day(self.today, days_ago);
        let input = SettlementInput {
            from_member: from,
            to_member: to,
            amount_cents,
            settled_on,
            note: note.map(str::to_string),
        };
        let at = evening(settled_on, self.next_minute());
        insert_settlement(self.conn, self.group_id, actor, &input, at).await?;
        Ok(())
    }
}

fn expense_input(
    description: &str,
    category: &str,
    amount_cents: i64,
    paid_by: Uuid,
    spent_on: NaiveDate,
    split: Split<'_>,
) -> ExpenseInput {
    let (method, splits): (&str, Vec<SplitInput>) = match split {
        Split::Equal(ids) => (
            "equal",
            ids.iter()
                .map(|id| SplitInput {
                    member_id: *id,
                    value: 0,
                })
                .collect(),
        ),
        Split::Exact(v) => ("exact", to_inputs(v)),
    };
    ExpenseInput {
        description: description.to_string(),
        amount_cents,
        paid_by,
        split_method: method.to_string(),
        category: Some(category.to_string()),
        spent_on,
        notes: None,
        splits,
    }
}

fn to_inputs(values: &[(Uuid, i64)]) -> Vec<SplitInput> {
    values
        .iter()
        .map(|(member_id, value)| SplitInput {
            member_id: *member_id,
            value: *value,
        })
        .collect()
}

async fn flat_share(
    conn: &mut PgConnection,
    today: NaiveDate,
    demo: &User,
    maya: &User,
    jonas: &User,
) -> Result<(), AppError> {
    let created = evening(day(today, 125), 5);
    let (group, alex) = insert_group(conn, demo.id, &demo.name, "Flat 4B", "EUR", created).await?;
    let maya_m = insert_member(
        &mut *conn,
        group.id,
        Some(maya.id),
        &maya.name,
        "member",
        created,
    )
    .await?;
    let jonas_m = insert_member(
        &mut *conn,
        group.id,
        Some(jonas.id),
        &jonas.name,
        "member",
        created,
    )
    .await?;
    let priya = insert_member(&mut *conn, group.id, None, "Priya", "member", created).await?;
    let everyone = [alex, maya_m, jonas_m, priya];

    let mut ctx = Ctx {
        conn,
        group_id: group.id,
        today,
        counter: 0,
    };

    // Four months of rent, internet, power and shopping. Alex has the big
    // room and pays a bit more rent.
    for month in 0..4 {
        let base = 120 - month * 30;
        ctx.expense(
            demo.id,
            base,
            "Rent",
            "rent",
            240_000,
            alex,
            Split::Exact(&[
                (alex, 72_000),
                (maya_m, 56_000),
                (jonas_m, 56_000),
                (priya, 56_000),
            ]),
        )
        .await?;
        ctx.expense(
            maya.id,
            base - 2,
            "Internet",
            "utilities",
            3_999,
            maya_m,
            Split::Equal(&everyone),
        )
        .await?;
        let power = [8_430, 9_215, 11_060, 10_480][month as usize];
        ctx.expense(
            jonas.id,
            base - 9,
            "Electricity",
            "utilities",
            power,
            jonas_m,
            Split::Equal(&everyone),
        )
        .await?;
        let shops = [
            ("Weekly shop", 6_342, alex),
            ("Groceries", 4_718, maya_m),
            ("Big shop at Lidl", 8_955, jonas_m),
        ];
        for (i, (description, amount, payer)) in shops.into_iter().enumerate() {
            let actor = if payer == alex {
                demo.id
            } else if payer == maya_m {
                maya.id
            } else {
                jonas.id
            };
            ctx.expense(
                actor,
                base - 4 - i as i64 * 7,
                description,
                "groceries",
                amount,
                payer,
                Split::Equal(&everyone),
            )
            .await?;
        }
    }

    ctx.expense(
        demo.id,
        96,
        "Cleaning supplies",
        "household",
        2_387,
        alex,
        Split::Equal(&everyone),
    )
    .await?;
    ctx.expense(
        maya.id,
        74,
        "New kettle",
        "household",
        3_499,
        maya_m,
        Split::Equal(&everyone),
    )
    .await?;
    ctx.expense(
        jonas.id,
        52,
        "Pizza night",
        "dining",
        5_640,
        jonas_m,
        Split::Equal(&[alex, maya_m, jonas_m]),
    )
    .await?;
    ctx.expense(
        demo.id,
        23,
        "Toilet paper & bin bags",
        "household",
        1_865,
        alex,
        Split::Equal(&everyone),
    )
    .await?;
    ctx.expense(
        maya.id,
        11,
        "Netflix (3 months)",
        "entertainment",
        4_197,
        maya_m,
        Split::Equal(&[alex, maya_m, jonas_m]),
    )
    .await?;

    // Everyone pays Alex back for rent most months; the last month is
    // still open, which is what the settle up suggestions are about.
    for (i, days_ago) in [116, 87, 58].into_iter().enumerate() {
        let note = ["Bank transfer", "Rent + bits", "Rent"][i];
        ctx.settle(maya.id, days_ago, maya_m, alex, 58_000, Some(note))
            .await?;
        ctx.settle(
            jonas.id,
            days_ago - 1,
            jonas_m,
            alex,
            56_000 + i as i64 * 2_500,
            Some(note),
        )
        .await?;
        ctx.settle(demo.id, days_ago - 2, priya, alex, 60_000, Some("Cash"))
            .await?;
    }
    Ok(())
}
