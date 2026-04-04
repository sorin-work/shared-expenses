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
    let sam = create_user(pool, "sam@example.com", "Sam", &hash).await?;

    let today = Utc::now().date_naive();
    let mut tx = pool.begin().await?;
    flat_share(&mut tx, today, &demo, &maya, &jonas).await?;
    trip(&mut tx, today, &demo, &maya).await?;
    household(&mut tx, today, &demo, &sam).await?;
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
    Percent(&'a [(Uuid, i64)]),
    Shares(&'a [(Uuid, i64)]),
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
        Split::Percent(v) => ("percent", to_inputs(v)),
        Split::Shares(v) => ("shares", to_inputs(v)),
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

    // Corrected after the receipt turned up, so the history has an edit.
    let plant = ctx
        .expense(
            demo.id,
            6,
            "Plants for the balcony",
            "household",
            3_000,
            alex,
            Split::Equal(&everyone),
        )
        .await?;
    let mut fixed = expense_input(
        "Plants for the balcony",
        "household",
        3_460,
        alex,
        day(today, 6),
        Split::Equal(&everyone),
    );
    fixed.notes = Some("Receipt says 34.60, not 30".into());
    let at = evening(day(today, 5), 40);
    expenses::update_expense(ctx.conn, group.id, plant, maya.id, &fixed, at).await?;

    // Entered twice by accident and removed again.
    let dup = ctx
        .expense(
            jonas.id,
            4,
            "Groceries",
            "groceries",
            4_210,
            jonas_m,
            Split::Equal(&everyone),
        )
        .await?;
    expenses::delete_expense(
        ctx.conn,
        group.id,
        dup,
        jonas.id,
        evening(day(today, 4), 50),
    )
    .await?;
    ctx.expense(
        jonas.id,
        4,
        "Groceries",
        "groceries",
        4_210,
        jonas_m,
        Split::Equal(&everyone),
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

async fn trip(
    conn: &mut PgConnection,
    today: NaiveDate,
    demo: &User,
    maya: &User,
) -> Result<(), AppError> {
    let created = evening(day(today, 58), 12);
    let (group, alex) = insert_group(
        conn,
        demo.id,
        &demo.name,
        "Lisbon long weekend",
        "EUR",
        created,
    )
    .await?;
    let maya_m = insert_member(
        &mut *conn,
        group.id,
        Some(maya.id),
        &maya.name,
        "member",
        created,
    )
    .await?;
    let tom = insert_member(&mut *conn, group.id, None, "Tom", "member", created).await?;
    let lena = insert_member(&mut *conn, group.id, None, "Lena", "member", created).await?;
    let all = [alex, maya_m, tom, lena];

    let mut ctx = Ctx {
        conn,
        group_id: group.id,
        today,
        counter: 0,
    };
    ctx.expense(
        demo.id,
        56,
        "Flights BER-LIS",
        "travel",
        68_396,
        alex,
        Split::Exact(&[
            (alex, 17_099),
            (maya_m, 17_099),
            (tom, 17_099),
            (lena, 17_099),
        ]),
    )
    .await?;
    // Tom and Lena shared the big bedroom: two shares for the couple.
    ctx.expense(
        maya.id,
        55,
        "Apartment in Alfama (3 nights)",
        "accommodation",
        51_000,
        maya_m,
        Split::Shares(&[(alex, 1), (maya_m, 1), (tom, 1), (lena, 1)]),
    )
    .await?;
    ctx.expense(
        demo.id,
        42,
        "Airport taxi",
        "transport",
        3_150,
        alex,
        Split::Equal(&all),
    )
    .await?;
    ctx.expense(
        maya.id,
        42,
        "Dinner at Taberna da Rua",
        "dining",
        11_780,
        maya_m,
        Split::Equal(&all),
    )
    .await?;
    ctx.expense(
        demo.id,
        41,
        "Tram 28 day passes",
        "transport",
        2_560,
        alex,
        Split::Equal(&all),
    )
    .await?;
    ctx.expense(
        demo.id,
        41,
        "Pastéis de nata",
        "dining",
        1_290,
        alex,
        Split::Equal(&[alex, maya_m, lena]),
    )
    .await?;
    ctx.expense(
        maya.id,
        41,
        "Oceanário tickets",
        "entertainment",
        8_800,
        maya_m,
        Split::Percent(&[(alex, 2_500), (maya_m, 2_500), (tom, 2_500), (lena, 2_500)]),
    )
    .await?;
    ctx.expense(
        demo.id,
        40,
        "Sintra day trip",
        "travel",
        9_640,
        alex,
        Split::Shares(&[(alex, 1), (maya_m, 1), (tom, 2)]),
    )
    .await?;
    ctx.expense(
        maya.id,
        40,
        "Seafood dinner",
        "dining",
        19_335,
        maya_m,
        Split::Equal(&all),
    )
    .await?;
    ctx.expense(
        demo.id,
        39,
        "Groceries for the apartment",
        "groceries",
        3_412,
        alex,
        Split::Equal(&all),
    )
    .await?;

    ctx.settle(
        demo.id,
        30,
        lena,
        alex,
        15_000,
        Some("Lena paid part on Revolut"),
    )
    .await?;
    Ok(())
}

async fn household(
    conn: &mut PgConnection,
    today: NaiveDate,
    demo: &User,
    sam: &User,
) -> Result<(), AppError> {
    let created = evening(day(today, 70), 30);
    let (group, alex) =
        insert_group(conn, demo.id, &demo.name, "Home with Sam", "GBP", created).await?;
    let sam_m = insert_member(
        &mut *conn,
        group.id,
        Some(sam.id),
        &sam.name,
        "member",
        created,
    )
    .await?;
    // Alex earns more, so shared bills are split 60/40.
    let sixty_forty = [(alex, 6_000), (sam_m, 4_000)];

    let mut ctx = Ctx {
        conn,
        group_id: group.id,
        today,
        counter: 0,
    };
    for (i, days_ago) in [65, 35, 5].into_iter().enumerate() {
        ctx.expense(
            demo.id,
            days_ago,
            "Council tax",
            "utilities",
            16_200,
            alex,
            Split::Percent(&sixty_forty),
        )
        .await?;
        let energy = [11_245, 12_890, 14_310][i];
        ctx.expense(
            sam.id,
            days_ago - 3,
            "Gas & electric",
            "utilities",
            energy,
            sam_m,
            Split::Percent(&sixty_forty),
        )
        .await?;
    }
    let shops = [
        (62, "Tesco", 7_643, sam_m),
        (55, "Sainsbury's", 5_419, alex),
        (47, "Tesco", 8_120, sam_m),
        (33, "Farmers market", 2_850, alex),
        (26, "Tesco", 6_975, sam_m),
        (12, "Sainsbury's", 9_302, alex),
        (3, "Tesco", 7_188, sam_m),
    ];
    for (days_ago, description, amount, payer) in shops {
        let actor = if payer == alex { demo.id } else { sam.id };
        ctx.expense(
            actor,
            days_ago,
            description,
            "groceries",
            amount,
            payer,
            Split::Equal(&[alex, sam_m]),
        )
        .await?;
    }
    ctx.expense(
        sam.id,
        44,
        "Thai takeaway",
        "dining",
        3_460,
        sam_m,
        Split::Equal(&[alex, sam_m]),
    )
    .await?;
    ctx.expense(
        demo.id,
        19,
        "Birthday present for Mum",
        "gifts",
        4_500,
        alex,
        Split::Exact(&[(alex, 2_250), (sam_m, 2_250)]),
    )
    .await?;
    ctx.expense(
        sam.id,
        8,
        "Cinema",
        "entertainment",
        2_680,
        sam_m,
        Split::Equal(&[alex, sam_m]),
    )
    .await?;

    ctx.settle(sam.id, 34, sam_m, alex, 8_000, None).await?;
    Ok(())
}
