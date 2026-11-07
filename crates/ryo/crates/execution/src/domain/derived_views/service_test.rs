//! The `derived_views` service's cases, on values alone.

use super::service::validate;
use super::types::contract::value::DerivedContract;
use super::types::{IdentityHash, ViewRequest};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::types::ColumnPlaces;
use ryo_derived::domain::request::types::DerivedKind;
use ryo_derived::domain::request::types::DerivedRequest as SemanticRequest;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The kind a contract is asked to serve.
type Input = DerivedKind;
type Value = ();
type Failure = OperatingError;

/// An equality contract over one number column.
fn equality() -> DerivedContract {
    let schemas = vec![vec![CellKind::Number]];
    let prepared = crate::domain::derived_views::fixtures::semantics().prepare(
        SemanticRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        schemas.clone(),
    );
    DerivedContract::issue(
        IdentityHash(0),
        std::sync::Arc::new(prepared),
        ViewRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        schemas,
        ColumnPlaces::default(),
    )
}

fn subject(expected: Input) -> Result<Value, Failure> {
    validate(expected, &equality())
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_contract_serves_the_kind_of_its_request"),
        word: Word::Succeeds,
        input: || DerivedKind::Equality,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_contract_refuses_a_kind_its_request_is_not"),
        word: Word::Refuses,
        input: || DerivedKind::Membership,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
