//! The `glob` comparisons: `glob_any` and `not glob_any`, a text matched against one glob or any of a list, by the `globbing`
//! concept.

use super::super::interface::ComparisonsItem;
use crate::domain::comparing::errors::ComparingError;
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::patterns::types::PatternMatcher;
use crate::domain::patterns::types::prepared::value::PreparedMatchers;
use crate::domain::table_scratch::types::ComparisonSlot;
use crate::domain::table_texts::types::Datum;
use ryo_matching::inbound::use_cases::matching::types::{
    CompiledTextGlobs, GlobPattern, MatchOutcome, SearchText,
};
use ryo_values::domain::plan_parts::service::columns_of;
use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, QueryLiteral, Test};

/// `glob`, one item of the `comparisons` collection, matching through the semantics its parent hands it.
pub struct GlobItem<'a>(pub PatternMatcher<'a>);

/// Uses a fixed operand's compiled matcher; a column operand remains row-dependent.
pub(crate) fn compiled_glob(
    comparison: &PlanComparison,
    left: &Datum<'_>,
    right: &Datum<'_>,
    prepared: Option<&PreparedMatchers>,
    compiled: &mut hashbrown::HashMap<ComparisonSlot, CompiledTextGlobs>,
    matcher: PatternMatcher<'_>,
) -> Option<PredicateOutcome> {
    let Datum::Text(text) = left else {
        return None;
    };
    let matching = matcher.0;
    let literal = match (prepared, &comparison.right) {
        (Some(prepared), Operand::Literal(QueryLiteral::Text(pattern))) => prepared.glob(
            comparison.test,
            crate::domain::table_texts::types::TextView(&pattern.0),
            matcher,
        ),
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
            PredicateOutcome::Holds
        } else {
            PredicateOutcome::Fails
        },
    )
}

/// Whether the text matches any of the globs, given as one text or as several: it holds when one does.
fn any(text: &Datum<'_>, globs: &Datum<'_>, matcher: PatternMatcher<'_>) -> PredicateOutcome {
    let Datum::Text(text) = text else {
        return PredicateOutcome::Fails;
    };
    let matching = matcher.0;
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
        PredicateOutcome::Holds
    } else {
        PredicateOutcome::Fails
    }
}

impl ComparisonsItem for GlobItem<'_> {
    fn holds(
        &self,
        test: Test,
        left: &Datum<'_>,
        right: &Datum<'_>,
    ) -> Result<PredicateOutcome, ComparingError> {
        match test {
            Test::GlobAny => Ok(any(left, right, self.0)),
            Test::NotGlobAny => Ok(match any(left, right, self.0) {
                PredicateOutcome::Holds => PredicateOutcome::Fails,
                PredicateOutcome::Fails => PredicateOutcome::Holds,
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
            | Test::Matches => Err(ComparingError::WrongTest),
        }
    }
}
