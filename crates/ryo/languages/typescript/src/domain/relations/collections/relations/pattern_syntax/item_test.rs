//! Cases for TypeScript `pattern_syntax` rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::PatternSyntaxItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_empty_class_is_one_element"),
        word: Word::Succeeds,
        input: || "/[]/;",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(5),
                    want_node(2),
                    want_name("pattern"),
                    want_name("[]"),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 2), (1, 4)),
                ],
                &[
                    want_node(6),
                    want_node(2),
                    want_name("disjunction"),
                    want_name("[]"),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 2), (1, 4)),
                ],
                &[
                    want_node(7),
                    want_node(2),
                    want_name("alternative"),
                    want_name("[]"),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 2), (1, 4)),
                ],
                &[
                    want_node(8),
                    want_node(2),
                    want_name("character-class"),
                    want_name("[]"),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 2), (1, 4)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_escaped_astral_character_has_its_code_point"),
        word: Word::Succeeds,
        input: || "/\\u{1F44D}/u;",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(5),
                    want_node(2),
                    want_name("pattern"),
                    want_name("\\u{1F44D}"),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 2), (1, 11)),
                ],
                &[
                    want_node(6),
                    want_node(2),
                    want_name("disjunction"),
                    want_name("\\u{1F44D}"),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 2), (1, 11)),
                ],
                &[
                    want_node(7),
                    want_node(2),
                    want_name("alternative"),
                    want_name("\\u{1F44D}"),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 2), (1, 11)),
                ],
                &[
                    want_node(8),
                    want_node(2),
                    want_name("character"),
                    want_name("\\u{1F44D}"),
                    want_number(128077),
                    want_file("item.ts"),
                    want_span((1, 2), (1, 11)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_indexed_reference_has_its_index"),
        word: Word::Succeeds,
        input: || "/(a)\\1/;",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(5),
                    want_node(2),
                    want_name("pattern"),
                    want_name("(a)\\1"),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 2), (1, 7)),
                ],
                &[
                    want_node(6),
                    want_node(2),
                    want_name("disjunction"),
                    want_name("(a)\\1"),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 2), (1, 7)),
                ],
                &[
                    want_node(7),
                    want_node(2),
                    want_name("alternative"),
                    want_name("(a)\\1"),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 2), (1, 7)),
                ],
                &[
                    want_node(8),
                    want_node(2),
                    want_name("capturing-group"),
                    want_name("(a)"),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 2), (1, 5)),
                ],
                &[
                    want_node(9),
                    want_node(2),
                    want_name("disjunction"),
                    want_name("a"),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 3), (1, 4)),
                ],
                &[
                    want_node(10),
                    want_node(2),
                    want_name("alternative"),
                    want_name("a"),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 3), (1, 4)),
                ],
                &[
                    want_node(11),
                    want_node(2),
                    want_name("character"),
                    want_name("a"),
                    want_number(97),
                    want_file("item.ts"),
                    want_span((1, 3), (1, 4)),
                ],
                &[
                    want_node(12),
                    want_node(2),
                    want_name("indexed-reference"),
                    want_name("\\1"),
                    want_number(1),
                    want_file("item.ts"),
                    want_span((1, 5), (1, 7)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_constructor_string_maps_to_its_source"),
        word: Word::Succeeds,
        input: || "RegExp(\"a\\\\.\");",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(10),
                    want_node(6),
                    want_name("pattern"),
                    want_name("a\\\\."),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 9), (1, 13)),
                ],
                &[
                    want_node(11),
                    want_node(6),
                    want_name("disjunction"),
                    want_name("a\\\\."),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 9), (1, 13)),
                ],
                &[
                    want_node(12),
                    want_node(6),
                    want_name("alternative"),
                    want_name("a\\\\."),
                    want_number(0),
                    want_file("item.ts"),
                    want_span((1, 9), (1, 13)),
                ],
                &[
                    want_node(13),
                    want_node(6),
                    want_name("character"),
                    want_name("a"),
                    want_number(97),
                    want_file("item.ts"),
                    want_span((1, 9), (1, 10)),
                ],
                &[
                    want_node(14),
                    want_node(6),
                    want_name("character"),
                    want_name("\\\\."),
                    want_number(46),
                    want_file("item.ts"),
                    want_span((1, 10), (1, 13)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_shadowed_constructor_writes_no_element"),
        word: Word::Succeeds,
        input: || "const RegExp = (p: string) => p; RegExp(\"a\");",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_empty_file_has_no_row"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];
fn subject(source: Input) -> Value {
    check::<PatternSyntaxItem>();
    reading::<PatternSyntaxItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
