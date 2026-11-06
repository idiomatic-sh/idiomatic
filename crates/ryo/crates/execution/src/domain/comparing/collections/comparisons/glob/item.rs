//! The `glob` comparisons: `glob_any` and `not glob_any`, a text matched against one glob or any of a list, by the `globbing`
//! concept.

use super::super::interface::ComparisonsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::Datum;
use ryo_matching::inbound::use_cases::matching::{
    interface::MatchingUseCase,
    types::{GlobPattern, MatchOutcome, SearchText},
    use_case::MatchingUseCaseImpl,
};
use ryo_values::domain::plan_parts::types::Test;

/// `glob`, one item of the `comparisons` collection.
pub(crate) struct GlobItem;

/// Whether the text matches any of the globs, given as one text or as several.
fn any(text: &Datum<'_>, globs: &Datum<'_>) -> bool {
    let Datum::Text(text) = text else {
        return false;
    };
    let matching = MatchingUseCaseImpl::new();
    match globs {
        Datum::Text(glob) => {
            matching.matches_text_glob(GlobPattern(glob.0), SearchText(text.0))
                == MatchOutcome::Matches
        }
        Datum::Texts(globs) => globs.iter().any(|glob| {
            matching.matches_text_glob(GlobPattern(&glob.0), SearchText(text.0))
                == MatchOutcome::Matches
        }),
        Datum::Unlocated | Datum::Number(_) | Datum::Flag(_) | Datum::Numbers(_) => false,
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
                true => Verdict::Holds,
                false => Verdict::Fails,
            }),
            Test::NotGlobAny => Ok(match any(left, right) {
                true => Verdict::Fails,
                false => Verdict::Holds,
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
