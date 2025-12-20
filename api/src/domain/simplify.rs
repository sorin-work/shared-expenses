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

/// Largest number of non-zero balances solved exactly (2^16 subsets).
pub const EXACT_LIMIT: usize = 16;

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

    if open.len() > EXACT_LIMIT {
        return settle_greedily(&open);
    }

    zero_sum_groups(&open)
        .iter()
        .flat_map(|group| settle_greedily(group))
        .collect()
}

/// Splits the balances into the largest possible number of zero-sum groups.
fn zero_sum_groups(open: &[(Uuid, i64)]) -> Vec<Vec<(Uuid, i64)>> {
    let n = open.len();
    if n == 0 {
        return Vec::new();
    }
    let full = (1usize << n) - 1;

    // sum[mask] = total balance of the members in mask
    let mut sum = vec![0i64; full + 1];
    for mask in 1..=full {
        let lowest = mask.trailing_zeros() as usize;
        sum[mask] = sum[mask & (mask - 1)] + open[lowest].1;
    }

    // best[mask] = most zero-sum groups the members in mask can form, given
    // that removing members one at a time closes a group whenever the
    // remaining set sums to zero.
    let mut best = vec![0u8; full + 1];
    for mask in 1..=full {
        let mut most = 0;
        let mut bits = mask;
        while bits != 0 {
            let bit = bits & bits.wrapping_neg();
            most = most.max(best[mask ^ bit]);
            bits ^= bit;
        }
        best[mask] = most + u8::from(sum[mask] == 0);
    }

    // Walk back from the full set to recover the groups.
    let mut groups = Vec::new();
    let mut current = Vec::new();
    let mut mask = full;
    while mask != 0 {
        let gained = u8::from(sum[mask] == 0);
        let index = (0..n)
            .find(|i| mask & (1 << i) != 0 && best[mask ^ (1 << i)] + gained == best[mask])
            .expect("some member leads to the optimum");
        current.push(open[index]);
        mask ^= 1 << index;
        if sum[mask] == 0 {
            // Keep the original member order inside each group.
            current.sort_by_key(|(id, _)| open.iter().position(|(o, _)| o == id));
            groups.push(std::mem::take(&mut current));
        }
    }
    groups
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
    fn finds_matching_pairs_greedy_would_miss() {
        // Greedy (biggest debtor pays biggest creditor) needs 4 transfers
        // here, but {3, -3} and {4, -2, -2} settle separately in 1 + 2.
        let input = balances(&[4, 3, -2, -2, -3]);
        let transfers = simplify(&input);
        assert_settles(&input, &transfers);
        assert_eq!(transfers.len(), 3);
        assert!(settle_greedily(&input).len() > transfers.len());
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

    #[test]
    fn large_groups_fall_back_to_greedy() {
        let mut values: Vec<i64> = (1..=10).map(|i| i * 100).collect();
        values.extend((1..=10).map(|i| -i * 100));
        let input = balances(&values);
        let transfers = simplify(&input);
        assert_settles(&input, &transfers);
        assert!(transfers.len() < input.len());
    }
}
