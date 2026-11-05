//! The `hosted` service's cases, on values alone.

use ryo_language::domain::symbols::types::Symbols;

use super::service::{lines, table_of_query};
use super::types::NumberedFiles;
use ryo_language::domain::files::types::FilePath;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
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
            let mut symbols = Symbols::default();
            table_of_query(&relation, NumberedFiles(&files), &mut (&mut symbols).into())
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

#[test]
fn typed_identity_distinguishes_text_separator_collisions_without_changing_presentation() {
    let relation = |first: &str, second: &str| HostRelation {
        name: HostName("host.values".into()),
        columns: ["first", "second"]
            .into_iter()
            .map(|name| HostColumn {
                name: HostColumnName(name.into()),
                kind: HostKind::Text,
            })
            .collect(),
        rows: vec![HostRow(vec![
            Cell::Text(first.into()),
            Cell::Text(second.into()),
        ])],
    };
    let left = relation("x\ttext:y", "z");
    let right = relation("x", "y\ttext:z");
    assert_eq!(
        lines(&[left.clone()]),
        lines(&[right.clone()]),
        "preserve the historical readable representation"
    );
    assert_ne!(
        ryo_values::domain::hosted::service::identity(&[left.clone()]),
        ryo_values::domain::hosted::service::identity(&[right])
    );
    assert_eq!(
        ryo_values::domain::hosted::service::identity(&[left.clone()]),
        ryo_values::domain::hosted::service::identity(&[left])
    );
    for (first, second) in [
        ("é\ntext:y", "z"),
        ("é", "y\ntext:z"),
        ("file:x", "number:1"),
        ("", "\t\n"),
    ] {
        assert_ne!(
            ryo_values::domain::hosted::service::identity(&[relation(first, second)]),
            ryo_values::domain::hosted::service::identity(&[relation(second, first)])
        );
    }
}
