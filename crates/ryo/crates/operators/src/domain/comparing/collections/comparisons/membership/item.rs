//! The `membership` comparisons: `in` and `not in`.

use super::super::interface::ComparisonsItem;
use crate::domain::comparing::errors::ComparingError;
use crate::domain::{table_scratch::types::SetMembership, contract::types::PredicateOutcome};
use crate::domain::table_texts::types::Datum;
use ryo_values::domain::plan_parts::types::Test;

/// `membership`, one item of the `comparisons` collection.
pub(crate) struct MembershipItem;

/// Whether `value` is one of the members.
fn member(value: &Datum<'_>, members: &Datum<'_>) -> SetMembership {
    let found = match (value, members) {
        (Datum::Number(number), Datum::Numbers(numbers)) => numbers.contains(number),
        (Datum::Text(text), Datum::Texts(texts)) => texts.iter().any(|each| &*each.0 == text.0),
        _ => false,
    };
    if found {
        SetMembership::In
    } else {
        SetMembership::Out
    }
}

impl ComparisonsItem for MembershipItem {
    fn holds(
        &self,
        test: Test,
        left: &Datum<'_>,
        right: &Datum<'_>,
    ) -> Result<PredicateOutcome, ComparingError> {
        match test {
            Test::In => Ok(match member(left, right) {
                SetMembership::In => PredicateOutcome::Holds,
                SetMembership::Out => PredicateOutcome::Fails,
            }),
            Test::NotIn => Ok(match member(left, right) {
                SetMembership::In => PredicateOutcome::Fails,
                SetMembership::Out => PredicateOutcome::Holds,
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
            | Test::GlobAny
            | Test::NotGlobAny
            | Test::StartsWith
            | Test::EndsWith
            | Test::Matches => Err(ComparingError::WrongTest),
        }
    }
}
