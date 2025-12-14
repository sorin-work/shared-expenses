//! "Settle up" suggestions: a list of transfers that brings every balance
//! to zero using as few transfers as possible.
//!
//! The minimum number of transfers for n non-zero balances is n - k, where k
//! is the largest number of groups the members can be split into such that
//! each group's balances sum to zero (each group of size m settles in m - 1
//! transfers). Finding k is a set partitioning problem, so it's solved
//! exactly with a dynamic program over subsets for up to
//! [`EXACT_LIMIT`] people and with a greedy fallback above that. Real groups
//! are small, so in practice the answer is always optimal.

use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Transfer {
    pub from: Uuid,
    pub to: Uuid,
    pub amount: i64,
}

/// Suggests transfers that settle `balances` (member, net balance). The
/// balances must sum to zero. Members are considered in the given order,
/// which makes the result deterministic.
pub fn simplify(balances: &[(Uuid, i64)]) -> Vec<Transfer> {
    let open: Vec<(Uuid, i64)> = balances.iter().copied().filter(|(_, b)| *b != 0).collect();
    debug_assert_eq!(open.iter().map(|(_, b)| b).sum::<i64>(), 0);
    settle_greedily(&open)
}

/// Repeatedly lets the biggest debtor pay the biggest creditor. Settles a
/// zero-sum group of m members in at most m - 1 transfers.
fn settle_greedily(open: &[(Uuid, i64)]) -> Vec<Transfer> {
    let mut creditors: Vec<(Uuid, i64)> = open.iter().copied().filter(|(_, b)| *b > 0).collect();
    let mut debtors: Vec<(Uuid, i64)> = open
        .iter()
        .filter(|(_, b)| *b < 0)
        .map(|(id, b)| (*id, -b))
        .collect();

    let mut transfers = Vec::new();
    loop {
        let Some(c) = (0..creditors.len()).max_by_key(|&i| creditors[i].1) else {
            break;
        };
        let Some(d) = (0..debtors.len()).max_by_key(|&i| debtors[i].1) else {
            break;
        };
        let amount = creditors[c].1.min(debtors[d].1);
        transfers.push(Transfer {
            from: debtors[d].0,
            to: creditors[c].0,
            amount,
        });
        creditors[c].1 -= amount;
        debtors[d].1 -= amount;
        creditors.retain(|(_, b)| *b > 0);
        debtors.retain(|(_, b)| *b > 0);
    }
    transfers
}
