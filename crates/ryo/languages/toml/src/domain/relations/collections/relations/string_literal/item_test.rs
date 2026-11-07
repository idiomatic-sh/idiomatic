//! The `string_literal` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::StringLiteralItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_flag, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_string_is_what_the_library_decoded"),
        word: Word::Succeeds,
        input: || "t = \"a\\tb\"\nu = 'raw\\n'\n\"key\" = \"\"\"\nml\"\"\"\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(7),
                    want_node(1),
                    want_name("a\tb"),
                    want_file(SHOWN),
                    want_span((1, 5), (1, 11)),
                    want_flag(false),
                ],
                &[
                    want_node(15),
                    want_node(9),
                    want_name("raw\\n"),
                    want_file(SHOWN),
                    want_span((2, 5), (2, 12)),
                    want_flag(false),
                ],
                &[
                    want_node(23),
                    want_node(17),
                    want_name("ml"),
                    want_file(SHOWN),
                    want_span((3, 9), (4, 6)),
                    want_flag(false),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_string_whose_escape_the_library_refuses_has_no_value"),
        word: Word::Succeeds,
        input: || "s = \"\\u+041\"\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[
                "the library refused the string's escapes, or did not decode this file in this worker",
            ],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<StringLiteralItem>();
    reading::<StringLiteralItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
