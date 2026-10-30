//! The `syntax_error` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::SyntaxErrorItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{MODULE_SHOWN, SHOWN, module_reading, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The file the item reads: a go source, or a module's `go.mod`.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// A go source.
    Source(&'static str),
    /// A `go.mod`.
    Module(&'static str),
}

type Input = Question;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_with_no_error_has_no_row"),
        word: Word::Succeeds,
        input: || Question::Source("package p\n"),
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("every_report_of_the_parser_is_a_row"),
        word: Word::Succeeds,
        input: || Question::Source("func f( {\n"),
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("syntax error: the parser could not read this text as go"),
                want_file(SHOWN),
                want_span((1, 1), (1, 10)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_statement_outside_a_function_is_a_row"),
        word: Word::Succeeds,
        input: || Question::Source("x++\n"),
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("syntax error: non-declaration statement outside function body"),
                want_file(SHOWN),
                want_span((1, 1), (1, 4)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_line_break_where_a_list_wants_a_comma_is_a_row"),
        word: Word::Succeeds,
        input: || Question::Source("var a = []int{\n\t3\n}\n"),
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name(
                    "syntax error: need a trailing comma before the newline that ends the list",
                ),
                want_file(SHOWN),
                want_span((3, 1), (3, 2)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_string_that_is_not_closed_is_an_error"),
        word: Word::Succeeds,
        input: || Question::Source("package p\nvar a = \"s\n"),
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("syntax error: the parser could not read this text as go"),
                want_file(SHOWN),
                want_span((2, 1), (2, 11)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_go_mod_block_that_is_not_closed_is_an_error"),
        word: Word::Succeeds,
        input: || Question::Module("module m\n\nrequire (\n\ta v1.0.0\n"),
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("syntax error: the block is not closed: its `)` is missing"),
                want_file(MODULE_SHOWN),
                want_span((3, 1), (5, 1)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_go_mod_with_its_blocks_closed_has_no_row"),
        word: Word::Succeeds,
        input: || Question::Module("module m\n\nrequire (\n\ta v1.0.0\n)\n"),
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];

fn subject(question: Input) -> Value {
    check::<SyntaxErrorItem>();
    match question {
        Question::Source(source) => reading::<SyntaxErrorItem>(source),
        Question::Module(source) => module_reading::<SyntaxErrorItem>(source),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
