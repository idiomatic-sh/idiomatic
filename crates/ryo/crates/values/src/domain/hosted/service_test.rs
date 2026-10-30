//! The `hosted` service's cases, on values alone.

use super::service::{declarations, kind_of};
use super::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::relations::types::CellKind;

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// How many columns the compiler is told of.
    Declared,
    /// What the three kinds of column hold.
    Kinds,
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// A count, or whether the answer was right.
type Value = usize;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_compiler_is_told_of_every_column_in_order"),
        word: Word::Succeeds,
        input: || Scenario::Declared,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("each_kind_of_column_holds_its_kind_of_cell"),
        word: Word::Succeeds,
        input: || Scenario::Kinds,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Declared => {
            let relation = HostRelation {
                name: HostName("owners".into()),
                columns: vec![
                    HostColumn {
                        name: HostColumnName("file".into()),
                        kind: HostKind::File,
                    },
                    HostColumn {
                        name: HostColumnName("owner".into()),
                        kind: HostKind::Text,
                    },
                ],
                rows: Vec::new(),
            };
            Ok(declarations(&[relation])
                .first()
                .map_or(0, |declared| declared.columns.len()))
        }
        Scenario::Kinds => Ok(usize::from(
            kind_of(HostKind::File) == CellKind::File
                && kind_of(HostKind::Text) == CellKind::Symbol
                && kind_of(HostKind::Number) == CellKind::Number,
        )),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
