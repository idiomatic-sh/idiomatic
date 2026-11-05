//! The `hosted` service's cases, on values alone.

use super::service::{declarations, identity, kind_of};
use super::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::records::types::Cell;
use ryo_relations::domain::relations::types::CellKind;

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// How many columns the compiler is told of.
    Declared,
    /// What the three kinds of column hold.
    Kinds,
    IdentityBoundaries,
    IdentityDeterminism,
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// A count, or whether the answer was right.
type Value = usize;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("typed_identity_distinguishes_separator_unicode_and_prefix_boundaries"),
        word: Word::Succeeds,
        input: || Scenario::IdentityBoundaries,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("typed_identity_is_deterministic_and_does_not_modify_host_rows"),
        word: Word::Succeeds,
        input: || Scenario::IdentityDeterminism,
        expect: Expect::Value(1),
    },
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
        Scenario::IdentityBoundaries => {
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
            let (left, right) = (("x\ttext:y", "z"), ("x", "y\ttext:z"));
            assert_eq!(
                format!("text:{}\ttext:{}", left.0, left.1),
                format!("text:{}\ttext:{}", right.0, right.1),
                "the chosen rows collide under the old delimiter-only presentation"
            );
            assert_ne!(
                identity(&[relation(left.0, left.1)]),
                identity(&[relation(right.0, right.1)])
            );
            for (first, second) in [
                ("é\ntext:y", "z"),
                ("é", "y\ntext:z"),
                ("file:x", "number:1"),
                ("", "\t\n"),
            ] {
                assert_ne!(
                    identity(&[relation(first, second)]),
                    identity(&[relation(second, first)]),
                    "ordered typed cell boundaries remain in the identity"
                );
            }
            Ok(1)
        }
        Scenario::IdentityDeterminism => {
            let relations = vec![HostRelation {
                name: HostName("host.values".into()),
                columns: vec![HostColumn {
                    name: HostColumnName("value".into()),
                    kind: HostKind::Text,
                }],
                rows: vec![
                    HostRow(vec![Cell::Text("é\t\nnumber:7".into())]),
                    HostRow(vec![Cell::Text("".into())]),
                ],
            }];
            let original = relations.clone();
            let expected = identity(&original);
            let actual = identity(&relations);
            assert_eq!(relations, original, "identity is read-only");
            assert_eq!(
                actual, expected,
                "equivalent independently owned inputs have the same identity"
            );
            assert_eq!(
                identity(&relations),
                actual,
                "repeated extraction is deterministic"
            );
            Ok(1)
        }
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
