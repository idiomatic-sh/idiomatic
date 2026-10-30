//! The `hosted` service's cases, on values alone.

use ryo_language::domain::symbols::types::Symbols;

use super::service::{lines, table_of};
use super::types::NumberedFiles;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
};
use ryo_values::domain::records::types::Cell;

/// What a row of the cases asks of the relation.
#[derive(Clone, Copy)]
enum Scenario {
    /// How many lines the digest of a configuration is made of.
    Lines,
    /// How many rows the table has, the host having named a file that the run lists and one that it does not.
    Rows,
}

/// What a row of the cases asks of the relation.
type Input = Scenario;
/// A count.
type Value = usize;
type Failure = ();

fn placed() -> HostRelation {
    HostRelation {
        name: HostName("host.placed".into()),
        columns: vec![
            HostColumn {
                name: HostColumnName("path".into()),
                kind: HostKind::File,
            },
            HostColumn {
                name: HostColumnName("spec".into()),
                kind: HostKind::Text,
            },
        ],
        rows: vec![
            HostRow(vec![Cell::File("a.rs".into()), Cell::Text("docs".into())]),
            HostRow(vec![
                Cell::File("not-listed.rs".into()),
                Cell::Text("docs".into()),
            ]),
        ],
    }
}

fn subject(scenario: Input) -> Value {
    let relation = placed();
    match scenario {
        Scenario::Lines => lines(&[relation]).len(),
        Scenario::Rows => {
            let files = [FilePath("a.rs".into()), FilePath("b.rs".into())];
            table_of(&relation, NumberedFiles(&files), &mut Symbols::default())
                .count
                .0
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_digest_has_a_line_for_the_relation_and_one_for_each_row"),
        word: Word::Succeeds,
        input: || Scenario::Lines,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("a_row_naming_a_file_the_run_does_not_list_is_no_row"),
        word: Word::Succeeds,
        input: || Scenario::Rows,
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
