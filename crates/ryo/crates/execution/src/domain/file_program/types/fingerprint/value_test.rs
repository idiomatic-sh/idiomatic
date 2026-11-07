//! The hash separates what the bucket must separate and equals itself for the same canonical operation.

use super::value::Fingerprint;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::file_program::types::{ProgramNodeId, NodeInput};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, RepeatedRows, StepId, StepScope};
use ryo_values::domain::plans::types::Operation;

#[test]
fn the_same_operation_edges_scope_and_phase_hash_alike_and_a_scope_changes_the_hash() {
    let operation = Operation::Project {
        from: Input::Step(StepId(0)),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Kept,
    };
    let edges = [NodeInput::Node(ProgramNodeId(1))];
    let first = Fingerprint::of(&operation, &edges, None, StepScope::PerFile);
    assert_eq!(
        first,
        Fingerprint::of(&operation, &edges, None, StepScope::PerFile)
    );
    assert_ne!(
        first,
        Fingerprint::of(&operation, &edges, Some(RuleIndex(0)), StepScope::PerFile)
    );
    assert_ne!(
        first,
        Fingerprint::of(&operation, &edges, None, StepScope::CrossFile)
    );
}
