//! The `mapping` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::MappingItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{MODULE_SHOWN, module_reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number};

/// The `go.mod` the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_file_holds_itself_and_a_require_block_is_held_by_the_file"),
        word: Word::Succeeds,
        input: || "require (\n\tb v1.0.0\n\ta v1.2.0\n)\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_name("implicit"),
                    want_number(0),
                    want_node(0),
                    want_file(MODULE_SHOWN),
                ],
                &[
                    want_node(1),
                    want_name("standard"),
                    want_number(0),
                    want_node(0),
                    want_file(MODULE_SHOWN),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_block_of_another_directive_is_no_mapping"),
        word: Word::Succeeds,
        input: || "module example.com/m\n\nexclude (\n\tx v0.1.0\n)\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(0),
                want_name("implicit"),
                want_number(0),
                want_node(0),
                want_file(MODULE_SHOWN),
            ]],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<MappingItem>();
    module_reading::<MappingItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
