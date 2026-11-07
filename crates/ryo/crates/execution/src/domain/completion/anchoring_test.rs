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
/// Where the rows a join places missing from one file lie.
#[derive(Debug, PartialEq, Eq)]
enum Placement {
    /// In the one file that was missing them, any row of it.
    MissingFile(u32),
    /// In the whole repository.
    Anywhere,
}
type Value = Placement;
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_key_of_a_relation_places_the_missing_rows_in_the_missing_file"),
        word: Word::Succeeds,
        input: || Operand::Relation,
        expect: Expect::Value(Placement::MissingFile(1)),
    },
    Case {
        name: CaseName("a_node_key_of_a_step_cannot_place_the_missing_rows"),
        word: Word::Succeeds,
        input: || Operand::Step,
        expect: Expect::Value(Placement::Anywhere),
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
    let coverage = output_coverage(
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
                coverage: crate::domain::completion::service::partial_any(vec![Some(FileId(1))]),
            },
        ],
    )?;
    match coverage {
        CoverageState::Partial { files, .. } if files == [Some(FileId(1))] => {
            Ok(Placement::MissingFile(1))
        }
        CoverageState::Partial { files, .. } if files == [None] => Ok(Placement::Anywhere),
        CoverageState::Complete
        | CoverageState::Partial { .. }
        | CoverageState::Inapplicable { .. } => Err(OperatingError::WrongOperation),
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
