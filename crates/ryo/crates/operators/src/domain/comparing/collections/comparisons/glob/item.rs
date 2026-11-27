//! The `glob` comparisons: `glob_any` and `not glob_any`, a text matched against one glob or any of a list, by the `globbing`
//! concept.

use super::super::interface::ComparisonsItem;
use crate::domain::comparing::errors::ComparingError;
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::patterns::service::glob_any;
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
#[derive(Debug)]
pub struct GlobItem<'a>(pub PatternMatcher<'a>);


impl ComparisonsItem for GlobItem<'_> {
    fn holds(
        &self,
        test: Test,
        left: &Datum<'_>,
        right: &Datum<'_>,
    ) -> Result<PredicateOutcome, ComparingError> {
        match test {
            Test::GlobAny => Ok(glob_any(left, right, self.0)),
            Test::NotGlobAny => Ok(match glob_any(left, right, self.0) {
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
