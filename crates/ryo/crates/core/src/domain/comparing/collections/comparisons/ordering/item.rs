//! The `ordering` comparisons: less than, greater than, and their or-equal.

use std::cmp::Ordering;

use super::super::interface::ComparisonsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::Datum;
use ryo_values::domain::plan_parts::types::Test;

/// `ordering`, one item of the `comparisons` collection.
pub(crate) struct OrderingItem;

impl ComparisonsItem for OrderingItem {
    fn holds(
        &self,
        test: Test,
        left: &Datum<'_>,
        right: &Datum<'_>,
    ) -> Result<Verdict, OperatingError> {
        if test == Test::NaturalLessThan {
            return Ok(match (left, right) {
                (Datum::Text(left), Datum::Text(right))
                    if natural_compare(left.0, right.0).is_lt() =>
                {
                    Verdict::Holds
                }
                _ => Verdict::Fails,
            });
        }
        // Numbers are ordered as numbers and texts byte by byte; anything else is never less or greater.
        let order = match (left, right) {
            (Datum::Number(left), Datum::Number(right)) => left.cmp(right),
            (Datum::Text(left), Datum::Text(right)) => left.0.cmp(right.0),
            _ => return Ok(Verdict::Fails),
        };
        match test {
            Test::LessThan
            | Test::LessThanOrEqual
            | Test::GreaterThan
            | Test::GreaterThanOrEqual => Ok(match (test, order) {
                (Test::LessThan, Ordering::Less)
                | (Test::LessThanOrEqual, Ordering::Less | Ordering::Equal)
                | (Test::GreaterThan, Ordering::Greater)
                | (Test::GreaterThanOrEqual, Ordering::Greater | Ordering::Equal) => Verdict::Holds,
                _ => Verdict::Fails,
            }),
            Test::Equal
            | Test::NotEqual
            | Test::NaturalLessThan
            | Test::FloatTextGreaterThanOrEqual
            | Test::NumericGreaterThanOrEqual
            | Test::IntegerSuccessor
            | Test::In
            | Test::NotIn
            | Test::GlobAny
            | Test::NotGlobAny
            | Test::StartsWith
            | Test::EndsWith
            | Test::Matches => Err(OperatingError::WrongTest),
        }
    }
}

/// Compare ASCII digit runs without integer conversion; all other characters use
/// Unicode scalar order. Unicode whitespace separates runs but has no own weight.
/// A zero-prefixed run compares lexically ("02" < "1", "01" < "010");
/// other runs compare length before digits ("2" < "10"). No normalization occurs.
pub(super) fn natural_compare(mut left: &str, mut right: &str) -> Ordering {
    loop {
        left = left.trim_start_matches(char::is_whitespace);
        right = right.trim_start_matches(char::is_whitespace);
        let (Some(a), Some(b)) = (left.chars().next(), right.chars().next()) else {
            return left.is_empty().cmp(&right.is_empty()).reverse();
        };
        if a.is_ascii_digit() && b.is_ascii_digit() {
            let a_len = left.bytes().take_while(u8::is_ascii_digit).count();
            let b_len = right.bytes().take_while(u8::is_ascii_digit).count();
            let (a_run, a_rest) = left.split_at(a_len);
            let (b_run, b_rest) = right.split_at(b_len);
            let order = if a == '0' || b == '0' {
                a_run.cmp(b_run)
            } else {
                a_len.cmp(&b_len).then_with(|| a_run.cmp(b_run))
            };
            if order != Ordering::Equal {
                return order;
            }
            left = a_rest;
            right = b_rest;
        } else {
            let order = a.cmp(&b);
            if order != Ordering::Equal {
                return order;
            }
            left = &left[a.len_utf8()..];
            right = &right[b.len_utf8()..];
        }
    }
}
