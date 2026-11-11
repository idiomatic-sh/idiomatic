//! Rule outputs outlive workspace drops, and evaluation errors leave no mutable state behind.
use super::super::super::fixtures::Root;
use super::fixtures::{Observed, Output, Program, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::rows::errors::RowsError::NoSuchColumn;
use std::borrow::Cow;

type Input = Program;
type Value = Observed;
type Failure = String;

/// The cell every output row holds.
const SEVEN: Cell = Cell::Number(Integer(7));

/// An owned number output of `cells`.
const fn output(cells: &'static [Cell]) -> Output {
    Output {
        kinds: Cow::Borrowed(&[CellKind::Number]),
        root: Root::complete(cells, cells.len()),
        allocation_kept: true,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("large_tiny_and_empty_rule_outputs_are_owned_beyond_workspace_drop"),
        word: Word::Succeeds,
        input: || Program::Sized,
        expect: Expect::Value(Observed::Owned([
            output(&[SEVEN; 4096]),
            output(&[SEVEN; 1]),
            output(&[]),
            output(&[SEVEN; 3]),
        ])),
    },
    Case {
        name: CaseName("evaluation_errors_do_not_leave_mutable_state_in_the_next_rule"),
        word: Word::Succeeds,
        input: || Program::FailingThenFollowing,
        expect: Expect::Value(Observed::Isolated {
            failed: Root::Failed(OperatingError::Table(NoSuchColumn { index: 99 })),
            next: output(&[SEVEN; 2]),
            repeated: Root::Failed(OperatingError::Table(NoSuchColumn { index: 99 })),
        }),
    },
];

/// Runs the program's rules on one file and finishes it.
fn subject(program: Input) -> Result<Value, Failure> {
    Ok(exercise(program))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
