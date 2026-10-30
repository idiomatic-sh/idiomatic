//! The `syntax_error` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::SyntaxErrorItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading, reading_commented};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// Whether the file is read as json or as json with comments, and the source the item reads.
type Input = (Dialect, &'static str);

/// How the path of the file is spelled, which decides how the language reads it.
#[derive(Clone, Copy)]
enum Dialect {
    Strict,
    Commented,
}
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_object_left_open_is_reported"),
        word: Word::Succeeds,
        input: || (Dialect::Strict, "{"),
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("Unterminated object on line 1 column 1"),
                want_file(SHOWN),
                want_span((1, 1), (1, 2)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_document_that_is_read_clean_has_no_error"),
        word: Word::Succeeds,
        input: || (Dialect::Strict, "{}"),
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_comment_in_a_file_of_json_is_reported"),
        word: Word::Succeeds,
        input: || (Dialect::Strict, "{} // c"),
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name(
                    "comments are not allowed in JSON (RFC 8259): only a file read as json with comments has them",
                ),
                want_file(SHOWN),
                want_span((1, 4), (1, 8)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_comment_in_a_file_of_json_with_comments_is_read_clean"),
        word: Word::Succeeds,
        input: || (Dialect::Commented, "{} // c"),
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];

fn subject((dialect, source): Input) -> Value {
    check::<SyntaxErrorItem>();
    match dialect {
        Dialect::Strict => reading::<SyntaxErrorItem>(source),
        Dialect::Commented => reading_commented::<SyntaxErrorItem>(source),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
