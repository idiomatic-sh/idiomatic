//! The `mapping` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::MappingItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_nested_array_of_tables_counts_within_its_member"),
        word: Word::Succeeds,
        input: || "[[fruit]]\n[[fruit.variety]]\n[[fruit]]\n[[fruit.variety]]\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_name("standard"),
                    want_number(0),
                    want_node(0),
                    want_file(SHOWN),
                ],
                &[
                    want_node(1),
                    want_name("sequence-member"),
                    want_number(0),
                    want_node(1),
                    want_file(SHOWN),
                ],
                &[
                    want_node(7),
                    want_name("sequence-member"),
                    want_number(0),
                    want_node(7),
                    want_file(SHOWN),
                ],
                &[
                    want_node(15),
                    want_name("sequence-member"),
                    want_number(1),
                    want_node(15),
                    want_file(SHOWN),
                ],
                &[
                    want_node(21),
                    want_name("sequence-member"),
                    want_number(0),
                    want_node(21),
                    want_file(SHOWN),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_path_through_an_array_of_tables_resolves_in_its_last_member"),
        word: Word::Succeeds,
        input: || "[[a]]\n[a.b.c]\n[[a]]\n[a.b.c]\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_name("standard"),
                    want_number(0),
                    want_node(0),
                    want_file(SHOWN),
                ],
                &[
                    want_node(1),
                    want_name("sequence-member"),
                    want_number(0),
                    want_node(1),
                    want_file(SHOWN),
                ],
                &[
                    want_node(7),
                    want_name("standard"),
                    want_number(0),
                    want_node(14),
                    want_file(SHOWN),
                ],
                &[
                    want_node(12),
                    want_name("implicit"),
                    want_number(0),
                    want_node(12),
                    want_file(SHOWN),
                ],
                &[
                    want_node(17),
                    want_name("sequence-member"),
                    want_number(1),
                    want_node(17),
                    want_file(SHOWN),
                ],
                &[
                    want_node(23),
                    want_name("standard"),
                    want_number(0),
                    want_node(30),
                    want_file(SHOWN),
                ],
                &[
                    want_node(28),
                    want_name("implicit"),
                    want_number(0),
                    want_node(28),
                    want_file(SHOWN),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_table_implied_then_written_is_one_mapping_and_keeps_its_kind"),
        word: Word::Succeeds,
        input: || "[a.b]\n[a]\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_name("standard"),
                    want_number(0),
                    want_node(0),
                    want_file(SHOWN),
                ],
                &[
                    want_node(1),
                    want_name("standard"),
                    want_number(0),
                    want_node(6),
                    want_file(SHOWN),
                ],
                &[
                    want_node(4),
                    want_name("implicit"),
                    want_number(0),
                    want_node(4),
                    want_file(SHOWN),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_pyproject"),
        word: Word::Succeeds,
        input: || include_str!("../../../../../../tests/fixtures/allocation/pyproject.toml"),
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_name("standard"),
                    want_number(0),
                    want_node(0),
                    want_file(SHOWN),
                ],
                &[
                    want_node(1),
                    want_name("standard"),
                    want_number(0),
                    want_node(4),
                    want_file(SHOWN),
                ],
                &[
                    want_node(30),
                    want_name("standard"),
                    want_number(0),
                    want_node(35),
                    want_file(SHOWN),
                ],
                &[
                    want_node(50),
                    want_name("standard"),
                    want_number(0),
                    want_node(57),
                    want_file(SHOWN),
                ],
                &[
                    want_node(53),
                    want_name("implicit"),
                    want_number(0),
                    want_node(53),
                    want_file(SHOWN),
                ],
                &[
                    want_node(55),
                    want_name("implicit"),
                    want_number(0),
                    want_node(55),
                    want_file(SHOWN),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<MappingItem>();
    reading::<MappingItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
