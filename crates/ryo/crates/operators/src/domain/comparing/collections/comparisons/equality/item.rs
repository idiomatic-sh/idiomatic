//! The `equality` comparisons: `equal` and `not equal`. A flag is equal to the word it is shown as, `true` or `false`, so a query can
//! write the flag it wants: `where import = "false"`.

use super::super::interface::ComparisonsItem;
use crate::domain::comparing::errors::ComparingError;
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::table_texts::types::Datum;
use ryo_relations::domain::relations::constants::{LOWERED, RAISED};
use ryo_values::domain::plan_parts::types::{FlagState, Test};

/// Whether two values are the same: by their kind and value, or a flag and the word it is shown as.
fn same(left: &Datum<'_>, right: &Datum<'_>) -> PredicateOutcome {
    let equal = match (left, right) {
        (Datum::Flag(flag), Datum::Text(text)) | (Datum::Text(text), Datum::Flag(flag)) => {
            text.0
                == match flag {
                    FlagState::Raised => RAISED,
                    FlagState::Lowered => LOWERED,
                }
        }
        _ => left == right,
    };
    if equal {
        PredicateOutcome::Holds
    } else {
        PredicateOutcome::Fails
    }
}

/// `equality`, one item of the `comparisons` collection.
pub struct EqualityItem;

impl ComparisonsItem for EqualityItem {
    fn holds(
        &self,
        test: Test,
        left: &Datum<'_>,
        right: &Datum<'_>,
    ) -> Result<PredicateOutcome, ComparingError> {
        match test {
            Test::Equal => Ok(same(left, right)),
            Test::NotEqual => Ok(match same(left, right) {
                PredicateOutcome::Holds => PredicateOutcome::Fails,
                PredicateOutcome::Fails => PredicateOutcome::Holds,
            }),
            Test::LessThan
            | Test::LessThanOrEqual
            | Test::GreaterThan
            | Test::GreaterThanOrEqual
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
            | Test::Matches => Err(ComparingError::WrongTest),
        }
    }
}
