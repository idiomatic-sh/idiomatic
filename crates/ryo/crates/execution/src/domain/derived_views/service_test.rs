//! The `derived_views` service's cases, on values alone.

use super::service::kind_of;
use super::types::ViewRequest;
use crate::domain::positions::types::Stride;
use ryo_derived::domain::contract::types::DerivedKind;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

type Input = ViewRequest;
type Value = DerivedKind;
type Failure = ();

fn subject(request: Input) -> Result<Value, Failure> {
    Ok(kind_of(&request))
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_raw_key_order_request_is_the_raw_key_order_kind"),
        word: Word::Succeeds,
        input: || ViewRequest::RawKeyOrder { stride: Stride(1) },
        expect: Expect::Value(DerivedKind::RawKeyOrder),
    },
    Case {
        name: CaseName("an_equality_request_is_the_equality_kind"),
        word: Word::Succeeds,
        input: || ViewRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        expect: Expect::Value(DerivedKind::Equality),
    },
    Case {
        name: CaseName("a_membership_request_is_the_membership_kind"),
        word: Word::Succeeds,
        input: || ViewRequest::Membership {
            column: ColumnIndex(0),
        },
        expect: Expect::Value(DerivedKind::Membership),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
