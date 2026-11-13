//! Plan-local step numbers normalize while the node edges keep their authored order.

use super::value::CanonicalOperation;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::file_program::types::{NodeInput, ProgramNodeId};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, RepeatedRows, StepId};
use ryo_values::domain::plans::types::Operation;

fn project(from: Input) -> Operation {
    Operation::Project {
        from,
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Kept,
    }
}

#[test]
fn a_step_input_reads_its_node_and_every_step_is_numbered_as_the_first() {
    let canonical = CanonicalOperation::of(
        &project(Input::Step(StepId(1))),
        &[None, Some(ProgramNodeId(7))],
        RuleIndex(0),
        &[],
    )
    .expect("the step has a node");
    assert_eq!(canonical.inputs, [NodeInput::Node(ProgramNodeId(7))]);
    assert_eq!(canonical.operation, project(Input::Step(StepId(0))));
}

#[test]
fn a_relation_input_stays_and_a_step_without_a_node_refuses() {
    let relation = Input::Relation(Relation::Function);
    let canonical = CanonicalOperation::of(&project(relation), &[], RuleIndex(0), &[])
        .expect("a relation needs no node");
    assert_eq!(canonical.inputs, [NodeInput::Relation(Relation::Function)]);
    assert_eq!(canonical.operation, project(relation));
    assert!(
        CanonicalOperation::of(&project(Input::Step(StepId(0))), &[None], RuleIndex(0), &[])
            .is_none()
    );
}
