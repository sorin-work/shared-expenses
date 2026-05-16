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

#[cfg(test)]
mod tests {
    use super::*;

    const A: Uuid = Uuid::from_u128(1);
    const B: Uuid = Uuid::from_u128(2);
    const C: Uuid = Uuid::from_u128(3);

    fn expense(paid_by: Uuid, shares: &[(Uuid, i64)]) -> ExpenseEntry {
        ExpenseEntry {
            paid_by,
            amount: shares.iter().map(|(_, a)| a).sum(),
            shares: shares
                .iter()
                .map(|(member_id, amount)| Share {
                    member_id: *member_id,
                    amount: *amount,
                })
                .collect(),
        }
    }

    fn nets(balances: &[MemberBalance]) -> Vec<i64> {
        balances.iter().map(|b| b.net).collect()
    }

    #[test]
    fn no_activity_means_everyone_is_settled() {
        let balances = compute(&[A, B, C], &[], &[]);
        assert_eq!(nets(&balances), vec![0, 0, 0]);
    }

    #[test]
    fn payer_is_owed_the_other_shares() {
        let balances = compute(
            &[A, B, C],
            &[expense(A, &[(A, 1000), (B, 1000), (C, 1000)])],
            &[],
        );
        assert_eq!(nets(&balances), vec![2000, -1000, -1000]);
        assert_eq!(balances[0].paid, 3000);
        assert_eq!(balances[0].share, 1000);
    }

    #[test]
    fn expenses_offset_each_other() {
        let balances = compute(
            &[A, B],
            &[
                expense(A, &[(A, 500), (B, 500)]),
                expense(B, &[(A, 500), (B, 500)]),
            ],
            &[],
        );
        assert_eq!(nets(&balances), vec![0, 0]);
    }

    #[test]
    fn settlements_move_balances_towards_zero() {
        let balances = compute(
            &[A, B],
            &[expense(A, &[(A, 500), (B, 500)])],
            &[SettlementEntry {
                from: B,
                to: A,
                amount: 500,
            }],
        );
        assert_eq!(nets(&balances), vec![0, 0]);
        assert_eq!(balances[1].sent, 500);
        assert_eq!(balances[0].received, 500);
    }

    #[test]
    fn partial_settlement_leaves_the_rest() {
        let balances = compute(
            &[A, B],
            &[expense(A, &[(B, 1000)])],
            &[SettlementEntry {
                from: B,
                to: A,
                amount: 400,
            }],
        );
        assert_eq!(nets(&balances), vec![600, -600]);
    }

    #[test]
    fn payer_who_is_not_a_participant() {
        let balances = compute(&[A, B, C], &[expense(A, &[(B, 700), (C, 300)])], &[]);
        assert_eq!(nets(&balances), vec![1000, -700, -300]);
    }

    #[test]
    fn unknown_members_are_appended() {
        let balances = compute(&[A], &[expense(A, &[(A, 100), (B, 100)])], &[]);
        assert_eq!(balances.len(), 2);
        assert_eq!(balances[1].member_id, B);
        assert_eq!(nets(&balances), vec![100, -100]);
    }

    #[test]
    fn balances_always_sum_to_zero() {
        let balances = compute(
            &[A, B, C],
            &[
                expense(A, &[(A, 334), (B, 333), (C, 333)]),
                expense(B, &[(B, 2500), (C, 7500)]),
                expense(C, &[(A, 1)]),
            ],
            &[SettlementEntry {
                from: C,
                to: B,
                amount: 4000,
            }],
        );
        assert_eq!(balances.iter().map(|b| b.net).sum::<i64>(), 0);
    }
}
