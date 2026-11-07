//! A step may join the rows of file f with the nodes of file g, keep (f's file, g's node) and so carry a node column that does not lie
//! in the file its row is partitioned by; with no per-column lineage a step's file or node key places nothing, so the missing file is
//! never narrowed to its own partition, while the same key of a relation the operand reads places the rows in the missing file.
use super::{
    service::output_coverage,
    types::{CoverageState, InputCoverage},
};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{CellKind, FileId, Relation};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, Input as PlanInput, JoinColumn, Side, StepId},
        plans::types::Operation,
    },
};

#[derive(Clone, Copy)]
enum Operand {
    Relation,
    Step,
}
type Input = Operand;
type Value = CoverageState;
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_key_of_a_relation_places_the_missing_rows_in_the_missing_file"),
        word: Word::Succeeds,
        input: || Operand::Relation,
        expect: Expect::Value(CoverageState::Partial {
            files: vec![Some(FileId(1))],
        }),
    },
    Case {
        name: CaseName("a_node_key_of_a_step_cannot_place_the_missing_rows"),
        word: Word::Succeeds,
        input: || Operand::Step,
        expect: Expect::Value(CoverageState::Partial { files: vec![None] }),
    },
];
fn subject(operand: Input) -> Result<Value, Failure> {
    let from = |at: u32| match operand {
        Operand::Relation => PlanInput::Relation(if at == 0 {
            Relation::Function
        } else {
            Relation::Identifier
        }),
        Operand::Step => PlanInput::Step(StepId(at)),
    };
    let join = Operation::Join {
        left: from(0),
        right: from(1),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            },
            JoinColumn {
                side: Side::Left,
                column: ColumnIndex(1),
            },
        ],
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    };
    let kinds = [CellKind::Node, CellKind::File];
    output_coverage(
        &join,
        &kinds,
        &[
            InputCoverage {
                ordinal: 0,
                kinds: &kinds,
                coverage: CoverageState::Complete,
            },
            InputCoverage {
                ordinal: 1,
                kinds: &kinds,
                coverage: CoverageState::Partial {
                    files: vec![Some(FileId(1))],
                },
            },
        ],
    )
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
