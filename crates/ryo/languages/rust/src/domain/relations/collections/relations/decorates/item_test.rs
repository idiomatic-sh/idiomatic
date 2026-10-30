//! The `decorates` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::DecoratesItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and the causes for which it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_attribute_decorates_its_function"),
        word: Word::Succeeds,
        input: || "#[inline]\nfn run() {}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_node(1),
                want_number(0),
                want_name("inline"),
                want_name(""),
                want_file(SHOWN),
                want_span((1, 1), (1, 10)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("each_attribute_of_each_declaration_is_a_row"),
        word: Word::Succeeds,
        input: || "#[derive(Debug, Clone)]\n#[serde(rename = \"x\")]\nstruct S {\n    #[allow(dead_code)]\n    field: u8,\n}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2),
                    want_node(1),
                    want_number(0),
                    want_name("derive"),
                    want_name("Debug, Clone"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 24)),
                ],
                &[
                    want_node(19),
                    want_node(1),
                    want_number(1),
                    want_name("serde"),
                    want_name("rename = \"x\""),
                    want_file(SHOWN),
                    want_span((2, 1), (2, 23)),
                ],
                &[
                    want_node(46),
                    want_node(45),
                    want_number(0),
                    want_name("allow"),
                    want_name("dead_code"),
                    want_file(SHOWN),
                    want_span((4, 5), (4, 24)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_value_and_a_condition_are_the_arguments_an_attribute_passes"),
        word: Word::Succeeds,
        input: || "#[doc = \"x\"]\n#[cfg(test)]\nfn run() {}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2),
                    want_node(1),
                    want_number(0),
                    want_name("doc"),
                    want_name("\"x\""),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 13)),
                ],
                &[
                    want_node(17),
                    want_node(1),
                    want_number(1),
                    want_name("cfg"),
                    want_name("test"),
                    want_file(SHOWN),
                    want_span((2, 1), (2, 13)),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<DecoratesItem>();
    reading::<DecoratesItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
