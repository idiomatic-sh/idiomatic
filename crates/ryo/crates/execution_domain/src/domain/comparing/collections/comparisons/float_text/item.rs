//! The `float-text` comparison: finite numbers written as text, compared as numbers.

use super::super::interface::ComparisonsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::Datum;
use ryo_values::domain::plan_parts::types::Test;

/// `float_text`, one item of the `comparisons` collection.
pub(crate) struct FloatTextItem;

/// A finite number a text writes.
#[derive(Clone, Copy, PartialEq, PartialOrd)]
struct WrittenNumber(f64);

/// The finite number a text writes, when it writes one.
fn number_of(datum: &Datum<'_>) -> Option<WrittenNumber> {
    match datum {
        Datum::Text(text) => match text.0.trim().parse::<f64>() {
            Ok(number) if number.is_finite() => Some(WrittenNumber(number)),
            // a text that writes no number is compared as text, which is the question asked
            Ok(_) | Err(_) => None,
        },
        Datum::Unlocated
        | Datum::Number(_)
        | Datum::Flag(_)
        | Datum::Numbers(_)
        | Datum::Texts(_) => None,
    }
}

impl ComparisonsItem for FloatTextItem {
    fn holds(
        &self,
        test: Test,
        left: &Datum<'_>,
        right: &Datum<'_>,
    ) -> Result<Verdict, OperatingError> {
        match test {
            Test::FloatTextGreaterThanOrEqual => Ok(match (number_of(left), number_of(right)) {
                (Some(left), Some(right)) if left >= right => Verdict::Holds,
                _ => Verdict::Fails,
            }),
            Test::NaturalLessThan
            | Test::Equal
            | Test::NumericGreaterThanOrEqual
            | Test::IntegerSuccessor
            | Test::NotEqual
            | Test::LessThan
            | Test::LessThanOrEqual
            | Test::GreaterThan
            | Test::GreaterThanOrEqual
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
