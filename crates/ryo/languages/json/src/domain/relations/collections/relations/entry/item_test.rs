//! The `entry` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::EntryItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{
    Reading, want_file, want_flag, want_name, want_node, want_number, want_span,
};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("members_are_entries_of_the_root_mapping"),
        word: Word::Succeeds,
        input: || "{\"a\":1,\"b\":\"x\"}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_name(""),
                    want_name("a"),
                    want_name("integer"),
                    want_node(4),
                    want_number(0),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 2), (1, 5)),
                    want_name("1"),
                    want_flag(false),
                    want_number(1),
                    want_name(""),
                    want_name(""),
                    want_number(10),
                ],
                &[
                    want_name("x"),
                    want_name("b"),
                    want_name("string"),
                    want_node(9),
                    want_number(1),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 8), (1, 11)),
                    want_name("\"x\""),
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
        name: CaseName("an_object_value_owns_the_entries_inside_it"),
        word: Word::Succeeds,
        input: || "{\"o\":{\"k\":false}}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_name(""),
                    want_name("k"),
                    want_name("boolean"),
                    want_node(9),
                    want_number(0),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((1, 7), (1, 10)),
                    want_name("false"),
                    want_flag(false),
                    want_number(0),
                    want_name(""),
                    want_name(""),
                    want_number(0),
                ],
                &[
                    want_name(""),
                    want_name("o"),
                    want_name("inline-mapping"),
                    want_node(4),
                    want_number(0),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((1, 2), (1, 5)),
                    want_name("{\"k\":false}"),
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
        name: CaseName("a_key_is_what_the_library_decoded"),
        word: Word::Succeeds,
        input: || "{\"q\\\"k\":1.5}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_name(""),
                want_name("q\"k"),
                want_name("float"),
                want_node(4),
                want_number(0),
                want_node(0),
                want_file(SHOWN),
                want_span((1, 2), (1, 8)),
                want_name("1.5"),
                want_flag(false),
                want_number(0),
                want_name("1.5"),
                want_name(""),
                want_number(0),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_null_value_is_an_entry_of_kind_null"),
        word: Word::Succeeds,
        input: || "{\"a\":null}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_name(""),
                want_name("a"),
                want_name("null"),
                want_node(4),
                want_number(0),
                want_node(0),
                want_file(SHOWN),
                want_span((1, 2), (1, 5)),
                want_name("null"),
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
    reading::<EntryItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
