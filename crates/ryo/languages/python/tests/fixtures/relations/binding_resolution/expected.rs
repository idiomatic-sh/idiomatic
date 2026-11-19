// Every BDD source of schedule-order resolution, one input each; rows recorded from a run and checked against the clauses.

use ryo::languages::contract::{
    BindingResolutionExpectedRow, Expected, expected_rows, file, name, node, number, span,
};

/// Independently specified complete rows for these inputs.
pub(super) fn expected() -> Expected {
    expected_rows::<BindingResolutionExpectedRow>(&[])
}
