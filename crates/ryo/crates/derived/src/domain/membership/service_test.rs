//! The `membership` service's cases, on values alone.

use super::service::build;
use super::types::{DerivedMembership, MarkSpace};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fault::types::DerivedFault;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};

#[derive(Clone, Copy)]
enum Scenario {
    ValuesAreDeduplicated,
    DenseMarksFollowTheVertices,
    AbsentColumn,
}
type Input = Scenario;
type Value = Vec<usize>;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("repeated_values_are_one_member"),
        word: Word::Succeeds,
        input: || Scenario::ValuesAreDeduplicated,
        expect: Expect::Value(Vec::new()),
    },
    Case {
        name: CaseName("dense_marks_follow_the_vertex_list"),
        word: Word::Succeeds,
        input: || Scenario::DenseMarksFollowTheVertices,
        expect: Expect::Value(Vec::new()),
    },
    Case {
        name: CaseName("a_column_the_rows_lack_is_refused"),
        word: Word::Refuses,
        input: || Scenario::AbsentColumn,
        expect: Expect::Error(DerivedFault::ShapeMismatch),
    },
];

fn table() -> Table {
    Table {
        kinds: vec![CellKind::Number],
        cells: [3, 3, 7].map(|key| Cell::Number(Integer(key))).to_vec(),
        count: RowCount(3),
    }
}

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let first = ColumnIndex(0);
    match input {
        Scenario::ValuesAreDeduplicated => match build(&table(), first, MarkSpace::Values)? {
            DerivedMembership::Values(values) => Ok(vec![0; values.len() - 2]),
            DerivedMembership::Dense(_) => Err(DerivedFault::WrongOperation),
        },
        Scenario::DenseMarksFollowTheVertices => {
            let vertices = [3, 4, 7].map(|key| Cell::Number(Integer(key)));
            match build(&table(), first, MarkSpace::Dense(&vertices))? {
                DerivedMembership::Dense(marks) => {
                    if marks.0.as_slice() == [true, false, true] {
                        Ok(Vec::new())
                    } else {
                        Err(DerivedFault::ShapeMismatch)
                    }
                }
                DerivedMembership::Values(_) => Err(DerivedFault::WrongOperation),
            }
        }
        Scenario::AbsentColumn => {
            build(&table(), ColumnIndex(9), MarkSpace::Values).map(|_| Vec::new())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
