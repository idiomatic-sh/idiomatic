//! The `glob` comparisons: `glob_any` and `not glob_any`, a text matched against one glob or any of a list, by the `globbing`
//! concept.

use super::super::interface::ComparisonsItem;
use crate::domain::matching::types::prepared::value::PreparedMatchers;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_scratch::types::ComparisonSlot;
use crate::domain::table_texts::types::Datum;
use ryo_matching::inbound::use_cases::matching::{
    compiled::CompiledTextGlobs,
    interface::MatchingUseCase,
    types::{GlobPattern, MatchOutcome, SearchText},
    use_case::MatchingUseCaseImpl,
};
use ryo_values::domain::plan_parts::service::columns_of;
use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, QueryLiteral, Test};

/// `glob`, one item of the `comparisons` collection.
pub(crate) struct GlobItem;

/// Uses a fixed operand's compiled matcher; a column operand remains row-dependent.
pub(crate) fn compiled_glob(
    comparison: &PlanComparison,
    left: &Datum<'_>,
    right: &Datum<'_>,
    prepared: Option<&PreparedMatchers>,
    compiled: &mut hashbrown::HashMap<ComparisonSlot, CompiledTextGlobs>,
) -> Option<Verdict> {
    let Datum::Text(text) = left else {
        return None;
    };
    let matching = MatchingUseCaseImpl::new();
    let literal = match (prepared, &comparison.right) {
        (Some(prepared), Operand::Literal(QueryLiteral::Text(pattern))) => {
            prepared.glob(
                comparison.test,
                crate::domain::table_texts::types::TextView(&pattern.0),
            )
        }
        _ => None,
    };
    let ready = match literal {
        Some(ready) => ready,
        None if !columns_of(&comparison.right).is_empty() => return None,
        None => {
            let key = ComparisonSlot(std::ptr::from_ref(comparison).addr());
            &*compiled.entry(key).or_insert_with(|| match right {
                Datum::Text(pattern) => matching.compile_text_globs(&[GlobPattern(pattern.0)]),
                Datum::Texts(patterns) => matching.compile_text_globs(
                    &patterns
                        .iter()
                        .map(|pattern| GlobPattern(pattern.0.as_ref()))
                        .collect::<Vec<_>>(),
                ),
                Datum::Unlocated | Datum::Number(_) | Datum::Flag(_) | Datum::Numbers(_) => {
                    CompiledTextGlobs::default()
                }
            })
        }
    };
    let outcome = matching.matches_text_globs(ready, SearchText(text.0));
    Some(
        if (outcome == MatchOutcome::Matches) == (comparison.test == Test::GlobAny) {
            Verdict::Holds
        } else {
            Verdict::Fails
        },
    )
}

/// Whether the text matches any of the globs, given as one text or as several: it holds when one does.
fn any(text: &Datum<'_>, globs: &Datum<'_>) -> Verdict {
    let Datum::Text(text) = text else {
        return Verdict::Fails;
    };
    let matching = MatchingUseCaseImpl::new();
    let matched = match globs {
        Datum::Text(glob) => {
            matching.matches_text_glob(GlobPattern(glob.0), SearchText(text.0))
                == MatchOutcome::Matches
        }
        Datum::Texts(globs) => globs.iter().any(|glob| {
            matching.matches_text_glob(GlobPattern(&glob.0), SearchText(text.0))
                == MatchOutcome::Matches
        }),
        Datum::Unlocated | Datum::Number(_) | Datum::Flag(_) | Datum::Numbers(_) => false,
    };
    if matched {
        Verdict::Holds
    } else {
        Verdict::Fails
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
            Test::GlobAny => Ok(any(left, right)),
            Test::NotGlobAny => Ok(match any(left, right) {
                Verdict::Holds => Verdict::Fails,
                Verdict::Fails => Verdict::Holds,
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
