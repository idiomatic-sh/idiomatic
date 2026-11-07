//! Numeric ordering and exact integer successor accept normalized text and integer cells.

use super::super::interface::ComparisonsItem;
use crate::domain::numeric_values::{
    service::{compare, integer_successor, read},
    types::{Numeric, NumberText, Succession},
};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::Datum;
use ryo_values::domain::plan_parts::types::Test;
use std::cmp::Ordering;

/// Read a numeric datum without converting flags, lists, or absent values.
fn number_of<'a>(datum: &Datum<'a>) -> Option<Numeric<'a>> {
    match datum {
        Datum::Text(text) => read(NumberText(text.0)),
        Datum::Number(number) => Some(Numeric::Whole(*number)),
        Datum::Unlocated | Datum::Flag(_) | Datum::Numbers(_) | Datum::Texts(_) => None,
    }
}

/// Exact integer/binary64 ordering.
pub(crate) struct NumericItem;

impl ComparisonsItem for NumericItem {
    fn holds(
        &self,
        test: Test,
        left: &Datum<'_>,
        right: &Datum<'_>,
    ) -> Result<Verdict, OperatingError> {
        match test {
            Test::NumericGreaterThanOrEqual => Ok(match (number_of(left), number_of(right)) {
                (Some(left), Some(right))
                    if matches!(
                        compare(left, right),
                        Some(Ordering::Equal | Ordering::Greater)
                    ) =>
                {
                    Verdict::Holds
                }
                _ => Verdict::Fails,
            }),
            Test::IntegerSuccessor => Ok(match (number_of(left), number_of(right)) {
                (Some(left), Some(right))
                    if integer_successor(left, right) == Succession::Successor =>
                {
                    Verdict::Holds
                }
                _ => Verdict::Fails,
            }),
            Test::Equal
            | Test::NotEqual
            | Test::LessThan
            | Test::LessThanOrEqual
            | Test::GreaterThan
            | Test::GreaterThanOrEqual
            | Test::NaturalLessThan
            | Test::FloatTextGreaterThanOrEqual
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
