//! Every operation variant routes as its meaning says; relabelled routes are refused.
use super::super::super::super::fixtures::operations::heads::operations;
use super::fixtures::{ROUTE_CASES, route_subject};
use crate::cases::{Word, run};
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::query_operators::types::OperationKind;
use std::collections::HashSet;

#[test]
fn routes_succeed() {
    run(ROUTE_CASES, Word::Succeeds, route_subject);
}
#[test]
fn routes_reject() {
    run(ROUTE_CASES, Word::Rejects, route_subject);
}

/// Guard: `meaning` and `kind_of` are exhaustive without a wildcard, so a 27th variant must
/// choose a route to compile; this asserts the catalog and the corpus agree on the count.
#[test]
fn every_catalog_kind_has_a_head_in_the_corpus() {
    let kinds = operations().iter().map(kind_of).collect::<HashSet<_>>();
    assert_eq!(kinds.len(), operations().len(), "one operation per kind");
    assert_eq!(kinds.len(), OperationKind::ALL.len());
}
