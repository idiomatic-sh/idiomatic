//! `directive` reader cases and the collection contract.
use super::super::fixtures::contract::check;
use super::item::DirectiveItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("empty_source_has_no_directive"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_file_starts_with_use_client"),
        word: Word::Succeeds,
        input: || "\"use client\";\nexport const a = 1;\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_file("item.js"),
                want_span((1, 1), (1, 14)),
                want_name("use client"),
                want_name(""),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_directive_is_its_spelling_not_its_decoded_value"),
        word: Word::Succeeds,
        input: || "\"use\\x20strict\";\nexport const a = 1;\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_file("item.js"),
                want_span((1, 1), (1, 17)),
                want_name("use\\x20strict"),
                want_name(""),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_function_body_starts_with_use_server"),
        word: Word::Succeeds,
        input: || "async function action() {\n  'use server';\n  return 1;\n}\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_file("item.js"),
                want_span((2, 3), (2, 16)),
                want_name("use server"),
                want_name(""),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_string_statement_after_code_is_no_directive"),
        word: Word::Succeeds,
        input: || "const a = 1;\n\"use client\";\nfunction f() {\n  const b = 2;\n  'use server';\n}\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_number_or_parenthesized_string_is_no_directive"),
        word: Word::Succeeds,
        input: || "1;\n(\"use client\");\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];
fn subject(source: Input) -> Value {
    check::<DirectiveItem>();
    reading::<DirectiveItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
