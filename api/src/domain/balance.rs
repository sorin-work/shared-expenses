//! Per-member balances for a group.
//!
//! A positive balance means the group owes the member money, a negative one
//! means the member owes the group. Balances of all members always add up to
//! zero.

use std::collections::HashMap;

use serde::Serialize;
use uuid::Uuid;

use super::split::Share;

#[derive(Debug, Clone)]
pub struct ExpenseEntry {
    pub paid_by: Uuid,
    pub amount: i64,
    pub shares: Vec<Share>,
}

#[derive(Debug, Clone, Copy)]
pub struct SettlementEntry {
    pub from: Uuid,
    pub to: Uuid,
    pub amount: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MemberBalance {
    pub member_id: Uuid,
    /// Total of the expenses this member paid for.
    pub paid: i64,
    /// Total of this member's shares of expenses.
    pub share: i64,
    /// Settlements this member paid to others.
    pub sent: i64,
    /// Settlements this member received from others.
    pub received: i64,
    /// paid - share + sent - received
    pub net: i64,
}

impl MemberBalance {
    fn new(member_id: Uuid) -> Self {
        Self {
            member_id,
            paid: 0,
            share: 0,
            sent: 0,
            received: 0,
            net: 0,
        }
    }
}

/// Computes balances for `members` (in that order). Entries that reference
/// a member who isn't in the list are still counted, and that member is
/// appended at the end, so the totals always balance.
pub fn compute(
    members: &[Uuid],
    expenses: &[ExpenseEntry],
    settlements: &[SettlementEntry],
) -> Vec<MemberBalance> {
    let mut order: Vec<Uuid> = members.to_vec();
    let mut by_member: HashMap<Uuid, MemberBalance> = members
        .iter()
        .map(|id| (*id, MemberBalance::new(*id)))
        .collect();

    fn entry<'a>(
        by_member: &'a mut HashMap<Uuid, MemberBalance>,
        order: &mut Vec<Uuid>,
        id: Uuid,
    ) -> &'a mut MemberBalance {
        by_member.entry(id).or_insert_with(|| {
            order.push(id);
            MemberBalance::new(id)
        })
    }

    for expense in expenses {
        entry(&mut by_member, &mut order, expense.paid_by).paid += expense.amount;
        for share in &expense.shares {
            entry(&mut by_member, &mut order, share.member_id).share += share.amount;
        }
    }
    for settlement in settlements {
        entry(&mut by_member, &mut order, settlement.from).sent += settlement.amount;
        entry(&mut by_member, &mut order, settlement.to).received += settlement.amount;
    }

    order
        .iter()
        .map(|id| {
            let mut balance = by_member[id];
            balance.net = balance.paid - balance.share + balance.sent - balance.received;
            balance
        })
        .collect()
}
