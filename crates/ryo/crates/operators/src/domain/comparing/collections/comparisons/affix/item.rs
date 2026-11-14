//! The `affix` comparisons: `starts_with` and `ends_with`, whether a text begins or ends with another.

use super::super::interface::ComparisonsItem;
use crate::domain::comparing::errors::ComparingError;
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::table_texts::types::Datum;
use ryo_values::domain::plan_parts::types::Test;

/// `affix`, one item of the `comparisons` collection.
pub struct AffixItem;

impl ComparisonsItem for AffixItem {
    fn holds(
        &self,
        test: Test,
        left: &Datum<'_>,
        right: &Datum<'_>,
    ) -> Result<PredicateOutcome, ComparingError> {
        // A side that is not text, such as a function of a name whose text is not held, is no ground for a match.
        let (Datum::Text(whole), Datum::Text(affix)) = (left, right) else {
            return match test {
                Test::StartsWith | Test::EndsWith => Ok(PredicateOutcome::Fails),
                Test::Equal
                | Test::NotEqual
                | Test::LessThan
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
                | Test::Matches => Err(ComparingError::WrongTest),
            };
        };
        let held = match test {
            Test::StartsWith => whole.0.starts_with(affix.0),
            Test::EndsWith => whole.0.ends_with(affix.0),
            Test::Equal
            | Test::NotEqual
            | Test::LessThan
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
            | Test::Matches => return Err(ComparingError::WrongTest),
        };
        Ok(if held { PredicateOutcome::Holds } else { PredicateOutcome::Fails })
    }
}
