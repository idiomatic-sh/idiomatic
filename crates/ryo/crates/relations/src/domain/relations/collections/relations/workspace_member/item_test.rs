//! The `workspace_member` relation's cases.

use super::super::interface::RelationsItem as _;
use super::item::{WorkspaceMember, WorkspaceMemberRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId};

/// A row to write: every column holds a number of its own, so that a column moved is a column found.
type Input = ();
/// The numbers the row is stored as, in column order.
type Value = [Bits; 2];
type Failure = ();

fn row() -> WorkspaceMemberRow {
    WorkspaceMemberRow {
        workspace: FileId(3),
        package: FileId(5),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_row_is_2_numbers_in_column_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(5)]),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(WorkspaceMember::words(row()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
