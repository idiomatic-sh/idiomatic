//! The `membership` comparisons: `in` and `not in`.

use super::super::interface::ComparisonsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::{SetMembership, Verdict};
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
    ) -> Result<Verdict, OperatingError> {
        match test {
            Test::In => Ok(match member(left, right) {
                SetMembership::In => Verdict::Holds,
                SetMembership::Out => Verdict::Fails,
            }),
            Test::NotIn => Ok(match member(left, right) {
                SetMembership::In => Verdict::Fails,
                SetMembership::Out => Verdict::Holds,
            }),
            Test::Equal
            | Test::NotEqual
            | Test::LessThan
            | Test::LessThanOrEqual
            | Test::GreaterThan
            | Test::GreaterThanOrEqual
            | Test::FloatTextGreaterThanOrEqual
            | Test::GlobAny
            | Test::NotGlobAny
            | Test::StartsWith
            | Test::EndsWith => Err(OperatingError::WrongTest),
            Test::Matches => Err(OperatingError::WrongTest),
        }
    }
}
