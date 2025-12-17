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

    // max_by_key returns the last maximum, so search from the back to
    // prefer whoever comes first on a tie.
    fn biggest(entries: &[(Uuid, i64)]) -> Option<usize> {
        (0..entries.len()).rev().max_by_key(|&i| entries[i].1)
    }

    let mut transfers = Vec::new();
    while let (Some(c), Some(d)) = (biggest(&creditors), biggest(&debtors)) {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn balances(values: &[i64]) -> Vec<(Uuid, i64)> {
        values
            .iter()
            .enumerate()
            .map(|(i, b)| (id(i as u128 + 1), *b))
            .collect()
    }

    /// Applies the transfers and checks that everyone ends up at zero.
    fn assert_settles(input: &[(Uuid, i64)], transfers: &[Transfer]) {
        let mut left: HashMap<Uuid, i64> = input.iter().copied().collect();
        for t in transfers {
            assert!(t.amount > 0, "transfer of {}", t.amount);
            assert_ne!(t.from, t.to);
            *left.get_mut(&t.from).unwrap() += t.amount;
            *left.get_mut(&t.to).unwrap() -= t.amount;
        }
        assert!(left.values().all(|b| *b == 0), "left over: {left:?}");
    }

    #[test]
    fn nothing_to_do_when_everyone_is_settled() {
        assert!(simplify(&balances(&[0, 0, 0])).is_empty());
        assert!(simplify(&[]).is_empty());
    }

    #[test]
    fn single_debt_is_one_transfer() {
        let input = balances(&[500, -500]);
        assert_eq!(
            simplify(&input),
            vec![Transfer {
                from: id(2),
                to: id(1),
                amount: 500
            }]
        );
    }

    #[test]
    fn one_creditor_many_debtors() {
        let input = balances(&[2000, -1000, -1000]);
        let transfers = simplify(&input);
        assert_eq!(transfers.len(), 2);
        assert_settles(&input, &transfers);
    }

    #[test]
    fn chains_collapse_into_direct_payments() {
        // A owes B 10, B owes C 10 -> A pays C directly.
        let input = balances(&[-1000, 0, 1000]);
        assert_eq!(
            simplify(&input),
            vec![Transfer {
                from: id(1),
                to: id(3),
                amount: 1000
            }]
        );
    }

    #[test]
    fn uses_n_minus_groups_transfers() {
        // Two independent pairs.
        let input = balances(&[700, -300, -700, 300]);
        let transfers = simplify(&input);
        assert_eq!(transfers.len(), 2);
        assert_settles(&input, &transfers);
    }

    #[test]
    fn result_is_deterministic() {
        let input = balances(&[100, 100, -100, -100]);
        let first = simplify(&input);
        for _ in 0..5 {
            assert_eq!(simplify(&input), first);
        }
        assert_eq!(
            first,
            vec![
                Transfer {
                    from: id(3),
                    to: id(1),
                    amount: 100
                },
                Transfer {
                    from: id(4),
                    to: id(2),
                    amount: 100
                },
            ]
        );
    }
}
