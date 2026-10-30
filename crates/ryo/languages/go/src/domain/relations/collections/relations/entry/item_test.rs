//! The `entry` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::EntryItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{MODULE_SHOWN, module_reading};
use ryo::languages::contract::{
    Reading, want_file, want_flag, want_name, want_node, want_number, want_span,
};

/// The `go.mod` the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_line_of_a_require_block_is_an_entry_in_its_place_in_the_block"),
        word: Word::Succeeds,
        input: || "require (\n\tb v1.0.0\n\ta v1.2.0\n)\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_name("v1.0.0"),
                    want_name("b"),
                    want_name("string"),
                    want_node(8),
                    want_number(0),
                    want_node(1),
                    want_file(MODULE_SHOWN),
                    want_span((2, 2), (2, 3)),
                    want_name("v1.0.0"),
                    want_flag(false),
                    want_number(0),
                    want_name(""),
                    want_name(""),
                    want_number(0),
                ],
                &[
                    want_name("v1.2.0"),
                    want_name("a"),
                    want_name("string"),
                    want_node(14),
                    want_number(1),
                    want_node(1),
                    want_file(MODULE_SHOWN),
                    want_span((3, 2), (3, 3)),
                    want_name("v1.2.0"),
                    want_flag(false),
                    want_number(0),
                    want_name(""),
                    want_name(""),
                    want_number(0),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName(
            "a_require_line_of_its_own_is_counted_among_the_files_requirements_and_other_directives_are_no_entry",
        ),
        word: Word::Succeeds,
        input: || "module example.com/m\n\ngo 1.22\n\nrequire \"q.example/x\" v0.1.0 // indirect\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_name("v0.1.0"),
                want_name("q.example/x"),
                want_name("string"),
                want_node(16),
                want_number(0),
                want_node(0),
                want_file(MODULE_SHOWN),
                want_span((5, 9), (5, 22)),
                want_name("v0.1.0"),
                want_flag(false),
                want_number(0),
                want_name(""),
                want_name(""),
                want_number(0),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName(
            "a_quoted_path_is_keyed_as_go_reads_it_and_a_quoted_version_keeps_its_spelling_as_its_text",
        ),
        word: Word::Succeeds,
        input: || "require \"example.com/\\u007a\" \"v1.0.0\"\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_name("v1.0.0"),
                want_name("example.com/z"),
                want_name("string"),
                want_node(4),
                want_number(0),
                want_node(0),
                want_file(MODULE_SHOWN),
                want_span((1, 9), (1, 29)),
                want_name("\"v1.0.0\""),
                want_flag(false),
                want_number(0),
                want_name(""),
                want_name(""),
                want_number(0),
            ]],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<EntryItem>();
    module_reading::<EntryItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
