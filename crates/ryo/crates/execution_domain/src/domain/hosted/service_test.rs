//! The `hosted` service's cases, on values alone.

use ryo_language::domain::symbols::types::Symbols;

use super::service::{lines, table_of_query};
use super::types::NumberedFiles;
use ryo_language::domain::files::types::FilePath;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
};
use ryo_values::domain::row_texts::types::Cell;

/// The texts of the one row of two text columns of a hosted relation.
type Texts = (&'static str, &'static str);

/// What a row asks of a hosted relation.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// The lines of the digest of `placed`.
    Lines,
    /// The rows of the table of `placed`, the run listing these files.
    Rows(&'static [&'static str]),
    /// Two relations of one row of two texts each, compared by their lines and their identities.
    Compare(Texts, Texts),
}

/// What a hosted relation answers.
#[derive(Debug, PartialEq)]
enum Answer {
    /// How many lines the digest is made of.
    Lines(usize),
    /// How many rows the table has.
    Rows(usize),
    /// Whether the two relations have the same lines and the same identity.
    Compared {
        same_lines: bool,
        same_identity: bool,
    },
}

type Input = Ask;
type Value = Answer;
type Failure = std::convert::Infallible;

/// A relation of two text columns and one row of these texts.
fn texts(row: Texts) -> HostRelation {
    HostRelation {
        name: HostName("host.values".into()),
        columns: ["first", "second"]
            .into_iter()
            .map(|name| HostColumn {
                name: HostColumnName(name.into()),
                kind: HostKind::Text,
            })
            .collect(),
        rows: vec![HostRow(vec![
            Cell::Text(row.0.into()),
            Cell::Text(row.1.into()),
        ])],
    }
}

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

fn subject(ask: Input) -> Result<Value, Failure> {
    use ryo_values::domain::hosted::service::identity;
    Ok(match ask {
        Ask::Lines => Answer::Lines(lines(&[placed()]).len()),
        Ask::Rows(listed) => {
            let files: Vec<FilePath> = listed.iter().map(|path| FilePath((*path).into())).collect();
            let mut symbols = Symbols::default();
            Answer::Rows(
                table_of_query(
                    &placed(),
                    NumberedFiles(&files),
                    &mut (&mut symbols).into(),
                    crate::domain::operating::types::AfterTheFiles::Before,
                )
                .count
                .0,
            )
        }
        Ask::Compare(left, right) => {
            let (left, right) = (texts(left), texts(right));
            Answer::Compared {
                same_lines: lines(&[left.clone()]) == lines(&[right.clone()]),
                same_identity: identity(&[left]) == identity(&[right]),
            }
        }
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_digest_has_a_line_for_the_relation_and_one_for_each_row"),
        word: Word::Succeeds,
        input: || Ask::Lines,
        expect: Expect::Value(Answer::Lines(3)),
    },
    Case {
        name: CaseName("a_row_naming_a_file_the_run_does_not_list_is_no_row"),
        word: Word::Succeeds,
        input: || Ask::Rows(&["a.rs", "b.rs"]),
        expect: Expect::Value(Answer::Rows(1)),
    },
    Case {
        name: CaseName(
            "typed_identity_distinguishes_text_separator_collisions_without_changing_presentation",
        ),
        word: Word::Succeeds,
        input: || Ask::Compare(("x\ttext:y", "z"), ("x", "y\ttext:z")),
        expect: Expect::Value(Answer::Compared {
            same_lines: true,
            same_identity: false,
        }),
    },
    Case {
        name: CaseName("one_relation_has_one_identity"),
        word: Word::Succeeds,
        input: || Ask::Compare(("x\ttext:y", "z"), ("x\ttext:y", "z")),
        expect: Expect::Value(Answer::Compared {
            same_lines: true,
            same_identity: true,
        }),
    },
    Case {
        name: CaseName("a_line_break_before_a_kind_does_not_collide"),
        word: Word::Succeeds,
        input: || Ask::Compare(("é\ntext:y", "z"), ("z", "é\ntext:y")),
        expect: Expect::Value(Answer::Compared {
            same_lines: false,
            same_identity: false,
        }),
    },
    Case {
        name: CaseName("a_line_break_in_the_second_text_does_not_collide"),
        word: Word::Succeeds,
        input: || Ask::Compare(("é", "y\ntext:z"), ("y\ntext:z", "é")),
        expect: Expect::Value(Answer::Compared {
            same_lines: false,
            same_identity: false,
        }),
    },
    Case {
        name: CaseName("texts_spelled_as_kinds_do_not_collide"),
        word: Word::Succeeds,
        input: || Ask::Compare(("file:x", "number:1"), ("number:1", "file:x")),
        expect: Expect::Value(Answer::Compared {
            same_lines: false,
            same_identity: false,
        }),
    },
    Case {
        name: CaseName("an_empty_text_beside_separators_does_not_collide"),
        word: Word::Succeeds,
        input: || Ask::Compare(("", "\t\n"), ("\t\n", "")),
        expect: Expect::Value(Answer::Compared {
            same_lines: false,
            same_identity: false,
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
