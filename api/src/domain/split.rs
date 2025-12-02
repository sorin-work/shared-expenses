//! Splitting an expense between its participants.
//!
//! All arithmetic is done on integer minor units. When an amount can't be
//! divided exactly, the leftover cents are handed out one at a time using the
//! largest remainder method, and ties go to the participant that comes first
//! in the input. Callers pass participants in a stable order (the group's
//! member order), so the same expense always splits the same way.

use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 100% expressed in basis points.
pub const PERCENT_TOTAL: i64 = 10_000;

/// Upper bound on participants in one expense, mostly to keep inputs sane.
pub const MAX_PARTICIPANTS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitMethod {
    /// Everyone pays the same. Inputs are ignored.
    Equal,
    /// Inputs are amounts in minor units and must add up to the total.
    Exact,
    /// Inputs are basis points (1% = 100) and must add up to 10 000.
    Percent,
    /// Inputs are weights, e.g. 2 shares for a couple and 1 for a single.
    Shares,
}

impl SplitMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "equal",
            Self::Exact => "exact",
            Self::Percent => "percent",
            Self::Shares => "shares",
        }
    }
}

impl fmt::Display for SplitMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SplitMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "equal" => Ok(Self::Equal),
            "exact" => Ok(Self::Exact),
            "percent" => Ok(Self::Percent),
            "shares" => Ok(Self::Shares),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Participant {
    pub member_id: Uuid,
    /// Meaning depends on the split method, see [`SplitMethod`].
    pub value: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Share {
    pub member_id: Uuid,
    pub amount: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SplitError {
    #[error("the amount must be greater than zero")]
    NonPositiveTotal,
    #[error("an expense needs at least one participant")]
    NoParticipants,
    #[error("an expense can have at most {MAX_PARTICIPANTS} participants")]
    TooManyParticipants,
    #[error("a member can only appear once in a split")]
    DuplicateParticipant,
    #[error("split values must be greater than zero")]
    NonPositiveValue,
    #[error("the exact amounts add up to {actual} but the expense is {expected}")]
    ExactMismatch { expected: i64, actual: i64 },
    #[error("percentages must add up to 100%")]
    PercentMismatch { actual: i64 },
    #[error("the values are too large")]
    Overflow,
}

/// Splits `total` between `participants` according to `method`.
///
/// The returned shares are in the same order as the input and always add up
/// to exactly `total`.
pub fn split(
    total: i64,
    method: SplitMethod,
    participants: &[Participant],
) -> Result<Vec<Share>, SplitError> {
    if total <= 0 {
        return Err(SplitError::NonPositiveTotal);
    }
    if participants.is_empty() {
        return Err(SplitError::NoParticipants);
    }
    if participants.len() > MAX_PARTICIPANTS {
        return Err(SplitError::TooManyParticipants);
    }
    let mut seen = HashSet::with_capacity(participants.len());
    if !participants.iter().all(|p| seen.insert(p.member_id)) {
        return Err(SplitError::DuplicateParticipant);
    }

    let weights: Vec<i64> = match method {
        SplitMethod::Equal => vec![1; participants.len()],
        SplitMethod::Exact | SplitMethod::Percent | SplitMethod::Shares => {
            if participants.iter().any(|p| p.value <= 0) {
                return Err(SplitError::NonPositiveValue);
            }
            participants.iter().map(|p| p.value).collect()
        }
    };

    let amounts = match method {
        SplitMethod::Exact => {
            let actual = checked_sum(&weights)?;
            if actual != total {
                return Err(SplitError::ExactMismatch {
                    expected: total,
                    actual,
                });
            }
            weights
        }
        SplitMethod::Percent => {
            let actual = checked_sum(&weights)?;
            if actual != PERCENT_TOTAL {
                return Err(SplitError::PercentMismatch { actual });
            }
            allocate(total, &weights)
        }
        SplitMethod::Equal | SplitMethod::Shares => allocate(total, &weights),
    };

    Ok(participants
        .iter()
        .zip(amounts)
        .map(|(p, amount)| Share {
            member_id: p.member_id,
            amount,
        })
        .collect())
}

fn checked_sum(values: &[i64]) -> Result<i64, SplitError> {
    values
        .iter()
        .try_fold(0i64, |acc, v| acc.checked_add(*v))
        .ok_or(SplitError::Overflow)
}

/// Distributes `total` proportionally to `weights` (all positive) using the
/// largest remainder method. The result always sums to `total`.
fn allocate(total: i64, weights: &[i64]) -> Vec<i64> {
    let weight_sum: i128 = weights.iter().map(|w| i128::from(*w)).sum();
    let total = i128::from(total);

    let mut amounts = Vec::with_capacity(weights.len());
    let mut remainders = Vec::with_capacity(weights.len());
    for (index, weight) in weights.iter().enumerate() {
        let exact = total * i128::from(*weight);
        amounts.push(exact / weight_sum);
        remainders.push((exact % weight_sum, index));
    }

    let assigned: i128 = amounts.iter().sum();
    let leftover =
        usize::try_from(total - assigned).expect("leftover is below the participant count");

    // Hand out the leftover cents one at a time.
    for amount in amounts.iter_mut().take(leftover) {
        *amount += 1;
    }

    amounts
        .into_iter()
        .map(|a| i64::try_from(a).expect("a share never exceeds the total"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: usize) -> Vec<Uuid> {
        (0..n).map(|i| Uuid::from_u128(i as u128 + 1)).collect()
    }

    fn people(values: &[i64]) -> Vec<Participant> {
        ids(values.len())
            .into_iter()
            .zip(values)
            .map(|(member_id, value)| Participant {
                member_id,
                value: *value,
            })
            .collect()
    }

    fn amounts(shares: &[Share]) -> Vec<i64> {
        shares.iter().map(|s| s.amount).collect()
    }

    #[test]
    fn equal_split_divides_evenly() {
        let shares = split(9000, SplitMethod::Equal, &people(&[0, 0, 0])).unwrap();
        assert_eq!(amounts(&shares), vec![3000, 3000, 3000]);
    }

    #[test]
    fn equal_split_gives_leftover_cents_to_the_first_participants() {
        let shares = split(1000, SplitMethod::Equal, &people(&[0, 0, 0])).unwrap();
        assert_eq!(amounts(&shares), vec![334, 333, 333]);

        let shares = split(1001, SplitMethod::Equal, &people(&[0, 0, 0])).unwrap();
        assert_eq!(amounts(&shares), vec![334, 334, 333]);
    }

    #[test]
    fn equal_split_ignores_input_values() {
        let shares = split(100, SplitMethod::Equal, &people(&[5, -3])).unwrap();
        assert_eq!(amounts(&shares), vec![50, 50]);
    }

    #[test]
    fn equal_split_of_less_cents_than_people() {
        let shares = split(2, SplitMethod::Equal, &people(&[0, 0, 0, 0])).unwrap();
        assert_eq!(amounts(&shares), vec![1, 1, 0, 0]);
    }

    #[test]
    fn shares_keep_member_ids_and_order() {
        let input = people(&[0, 0]);
        let shares = split(10, SplitMethod::Equal, &input).unwrap();
        assert_eq!(shares[0].member_id, input[0].member_id);
        assert_eq!(shares[1].member_id, input[1].member_id);
    }

    #[test]
    fn exact_split_uses_the_given_amounts() {
        let shares = split(5000, SplitMethod::Exact, &people(&[1250, 3750])).unwrap();
        assert_eq!(amounts(&shares), vec![1250, 3750]);
    }

    #[test]
    fn exact_split_must_add_up() {
        let err = split(5000, SplitMethod::Exact, &people(&[1250, 3000])).unwrap_err();
        assert_eq!(
            err,
            SplitError::ExactMismatch {
                expected: 5000,
                actual: 4250
            }
        );
    }

    #[test]
    fn percent_split_must_total_one_hundred() {
        let err = split(1000, SplitMethod::Percent, &people(&[5000, 4000])).unwrap_err();
        assert_eq!(err, SplitError::PercentMismatch { actual: 9000 });
    }

    #[test]
    fn shares_split_is_proportional() {
        // A couple (2 shares) and a single (1 share) splitting 90.00
        let shares = split(9000, SplitMethod::Shares, &people(&[2, 1])).unwrap();
        assert_eq!(amounts(&shares), vec![6000, 3000]);
    }

    #[test]
    fn rejects_non_positive_totals() {
        assert_eq!(
            split(0, SplitMethod::Equal, &people(&[0])).unwrap_err(),
            SplitError::NonPositiveTotal
        );
        assert_eq!(
            split(-10, SplitMethod::Equal, &people(&[0])).unwrap_err(),
            SplitError::NonPositiveTotal
        );
    }

    #[test]
    fn rejects_empty_and_duplicate_participants() {
        assert_eq!(
            split(10, SplitMethod::Equal, &[]).unwrap_err(),
            SplitError::NoParticipants
        );
        let id = Uuid::from_u128(7);
        let dup = [
            Participant {
                member_id: id,
                value: 1,
            },
            Participant {
                member_id: id,
                value: 1,
            },
        ];
        assert_eq!(
            split(10, SplitMethod::Shares, &dup).unwrap_err(),
            SplitError::DuplicateParticipant
        );
    }

    #[test]
    fn rejects_zero_and_negative_values() {
        for method in [
            SplitMethod::Exact,
            SplitMethod::Percent,
            SplitMethod::Shares,
        ] {
            assert_eq!(
                split(100, method, &people(&[100, 0])).unwrap_err(),
                SplitError::NonPositiveValue
            );
            assert_eq!(
                split(100, method, &people(&[200, -100])).unwrap_err(),
                SplitError::NonPositiveValue
            );
        }
    }
}
