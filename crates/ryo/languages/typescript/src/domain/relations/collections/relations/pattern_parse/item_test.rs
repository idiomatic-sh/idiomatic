//! Cases for TypeScript `pattern_parse` rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::PatternParseItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_literal_is_parsed_with_its_flags"),
        word: Word::Succeeds,
        input: || "const a = /a+/gu;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(10),
                want_name("gu"),
                want_name("parsed"),
                want_name(""),
                want_file("item.ts"),
                want_span((1, 11), (1, 17)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_global_constructor_string_that_does_not_parse_is_malformed"),
        word: Word::Succeeds,
        input: || "new RegExp(\"[\");",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(8),
                want_name(""),
                want_name("malformed"),
                want_name("Invalid regular expression: Unterminated character class"),
                want_file("item.ts"),
                want_span((1, 13), (1, 14)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("flags_an_argument_does_not_settle_are_unknown"),
        word: Word::Succeeds,
        input: || "RegExp(\"b\", flags);",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(6),
                want_name("unknown"),
                want_name("parsed"),
                want_name(""),
                want_file("item.ts"),
                want_span((1, 8), (1, 11)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_shadowed_constructor_that_refuses_is_checked"),
        word: Word::Succeeds,
        input: || "const RegExp = (p: string) => p; RegExp(\"[\");",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(32),
                want_name(""),
                want_name("malformed"),
                want_name("Invalid regular expression: Unterminated character class"),
                want_file("item.ts"),
                want_span((1, 42), (1, 43)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_shadowed_constructor_that_parses_carries_nothing"),
        word: Word::Succeeds,
        input: || "const RegExp = (p: string) => p; RegExp(\"a\");",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("flags_alone_are_checked_when_the_pattern_is_no_string"),
        word: Word::Succeeds,
        input: || "RegExp(p, \"gz\"); RegExp(p, \"g\");",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(10),
                want_name("gz"),
                want_name("malformed-flags"),
                want_name("Invalid regular expression: Unknown flag: `z` found"),
                want_file("item.ts"),
                want_span((1, 13), (1, 14)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_nested_call_keeps_the_outer_flags_carrier"),
        word: Word::Succeeds,
        input: || "const a = new RegExp(RegExp(\"[\").source, \"z\");",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(21),
                    want_name(""),
                    want_name("malformed"),
                    want_name("Invalid regular expression: Unterminated character class"),
                    want_file("item.ts"),
                    want_span((1, 30), (1, 31)),
                ],
                &[
                    want_node(29),
                    want_name("z"),
                    want_name("malformed-flags"),
                    want_name("Invalid regular expression: Unknown flag: `z` found"),
                    want_file("item.ts"),
                    want_span((1, 43), (1, 44)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_string_passed_elsewhere_carries_no_pattern"),
        word: Word::Succeeds,
        input: || "f(\"[\");",
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
    check::<PatternParseItem>();
    reading::<PatternParseItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
