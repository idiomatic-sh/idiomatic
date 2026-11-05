//! The `glob` comparisons: `glob_any` and `not glob_any`, a text matched against one glob or any of a list, by the `globbing`
//! concept.

use super::super::interface::ComparisonsItem;
use crate::domain::globbing::service::matches;
use crate::domain::globbing::types::{GlobMatch, GlobText, Subject};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::Datum;
use ryo_values::domain::plan_parts::types::Test;

/// `glob`, one item of the `comparisons` collection.
pub(crate) struct GlobItem;

/// Whether the text matches any of the globs, given as one text or as several.
fn any(text: &Datum<'_>, globs: &Datum<'_>) -> GlobMatch {
    let Datum::Text(text) = text else {
        return GlobMatch::Differs;
    };
    let subject = Subject(text.0);
    match globs {
        Datum::Text(glob) => matches(GlobText(glob.0), subject),
        Datum::Texts(globs) => {
            if globs
                .iter()
                .any(|glob| matches(GlobText(&glob.0), subject) == GlobMatch::Matches)
            {
                GlobMatch::Matches
            } else {
                GlobMatch::Differs
            }
        }
        Datum::Unlocated | Datum::Number(_) | Datum::Flag(_) | Datum::Numbers(_) => {
            GlobMatch::Differs
        }
    }
}

impl ComparisonsItem for GlobItem {
    fn holds(
        &self,
        test: Test,
        left: &Datum<'_>,
        right: &Datum<'_>,
    ) -> Result<Verdict, OperatingError> {
        match test {
            Test::GlobAny => Ok(match any(left, right) {
                GlobMatch::Matches => Verdict::Holds,
                GlobMatch::Differs => Verdict::Fails,
            }),
            Test::NotGlobAny => Ok(match any(left, right) {
                GlobMatch::Matches => Verdict::Fails,
                GlobMatch::Differs => Verdict::Holds,
            }),
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
            | Test::StartsWith
            | Test::EndsWith
            | Test::Matches => Err(OperatingError::WrongTest),
        }
    }
}
