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
            | Test::FloatTextGreaterThanOrEqual
            | Test::In
            | Test::NotIn
            | Test::GlobAny
            | Test::NotGlobAny
            | Test::StartsWith
            | Test::EndsWith => Err(OperatingError::WrongTest),
            Test::Matches => Err(OperatingError::WrongTest),
        }
    }
}
