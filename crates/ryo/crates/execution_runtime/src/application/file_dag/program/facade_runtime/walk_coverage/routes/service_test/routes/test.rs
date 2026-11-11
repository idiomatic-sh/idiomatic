//! Every operation variant routes as its meaning says; relabelled routes are refused.
use super::super::super::super::fixtures::operations::heads::operations;
use crate::cases::{Case, Word, run};
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::query_operators::types::OperationKind;
use std::collections::HashSet;

/// The input, value and failure of a row, as the group's fixtures state them.
type Input = super::fixtures::Input;
type Value = super::fixtures::Value;
type Failure = super::fixtures::Failure;

/// The group's rows, stated with its fixtures.
const CASES: &[Case<Input, Value, Failure>] = super::fixtures::ROUTE_CASES;

/// Runs one row through the scenario its fixtures build.
fn subject(input: Input) -> Result<Value, Failure> {
    super::fixtures::route_subject(input)
}

#[test]
fn routes_succeed() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn routes_reject() {
    run(CASES, Word::Rejects, subject);
}

/// Guard: `meaning` and `kind_of` are exhaustive without a wildcard, so a 27th variant must
/// choose a route to compile; this asserts the catalog and the corpus agree on the count.
#[test]
fn every_catalog_kind_has_a_head_in_the_corpus() {
    let kinds = operations().iter().map(kind_of).collect::<HashSet<_>>();
    assert_eq!(kinds.len(), operations().len(), "one operation per kind");
    assert_eq!(kinds.len(), OperationKind::ALL.len());
}
