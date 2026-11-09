//! A closure or cycles step keeps its vertices of one kind: from a node to a file it is refused when its statement is lowered.
use super::super::fixtures::column;
use crate::cases::{Case, CaseName, Expect, Word, lowering_builder, run};
use crate::domain::lowering::errors::LoweringError;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

/// A closure or cycles step over the functions (id, name, file, span) from the node to the column at `destination`.
fn walked(cycles: bool, destination: u32) -> Result<(), LoweringError> {
    let mut builder = lowering_builder();
    let (source, destination) = (parts::ColumnIndex(0), parts::ColumnIndex(destination));
    let edges = parts::Input::Relation(Relation::Function);
    let operation = if cycles {
        plans::Operation::Cycles {
            edges,
            source,
            destination,
        }
    } else {
        plans::Operation::Closure {
            edges,
            keys: vec![],
            source,
            destination,
        }
    };
    crate::domain::planning::service::push(
        &mut builder,
        None,
        operation,
        vec![column(CellKind::Node), column(CellKind::Node)],
    );
    super::super::service::refuse_mixed_vertices(
        &builder,
        ryo_values::domain::plan_parts::types::StepId(0),
    )
}

/// A closure or cycles step from a node to a file is refused when the statement is lowered; from a node to a node it is kept.
fn a_walk_between_two_kinds_is_refused() {
    for cycles in [true, false] {
        assert!(
            matches!(walked(cycles, 2), Err(LoweringError::MixedVertices { .. })),
            "a walk from a node to a file is kept (cycles: {cycles})"
        );
        assert_eq!(
            walked(cycles, 0),
            Ok(()),
            "a walk from a node to a node is refused (cycles: {cycles})"
        );
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_closure_or_cycles_between_two_kinds_is_refused_when_lowered"),
    word: Word::Succeeds,
    input: || a_walk_between_two_kinds_is_refused,
    expect: Expect::Value(()),
}];

fn subject(check: Input) -> Result<Value, Failure> {
    check();
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
