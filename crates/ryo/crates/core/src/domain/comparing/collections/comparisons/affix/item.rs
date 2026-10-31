//! The `affix` comparisons: `starts_with` and `ends_with`, whether a text begins or ends with another.

use super::super::interface::ComparisonsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::Datum;
use ryo_values::domain::plan_parts::types::Test;

/// `affix`, one item of the `comparisons` collection.
pub(crate) struct AffixItem;

impl ComparisonsItem for AffixItem {
    fn holds(
        &self,
        test: Test,
        left: &Datum<'_>,
        right: &Datum<'_>,
    ) -> Result<Verdict, OperatingError> {
        // A side that is not text, such as a function of a name whose text is not held, is no ground for a match.
        let (Datum::Text(whole), Datum::Text(affix)) = (left, right) else {
            return match test {
                Test::StartsWith | Test::EndsWith => Ok(Verdict::Fails),
                Test::Equal
                | Test::NotEqual
                | Test::LessThan
                | Test::LessThanOrEqual
                | Test::GreaterThan
                | Test::GreaterThanOrEqual
                | Test::FloatTextGreaterThanOrEqual
                | Test::In
                | Test::NotIn
                | Test::GlobAny
                | Test::NotGlobAny => Err(OperatingError::WrongTest),
                Test::Matches => Err(OperatingError::WrongTest),
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
            | Test::FloatTextGreaterThanOrEqual
            | Test::In
            | Test::NotIn
            | Test::GlobAny
            | Test::NotGlobAny => return Err(OperatingError::WrongTest),
            Test::Matches => return Err(OperatingError::WrongTest),
        };
        Ok(if held { Verdict::Holds } else { Verdict::Fails })
    }
}
