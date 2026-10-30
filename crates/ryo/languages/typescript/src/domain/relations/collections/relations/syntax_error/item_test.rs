//! Cases for TypeScript `syntax_error` rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::SyntaxErrorItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("parser_diagnostic"),
        word: Word::Succeeds,
        input: || "const = ;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name("Unexpected token"),
                want_file("item.ts"),
                want_span((1, 7), (1, 8)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("infer_is_only_valid_in_a_conditional_extends_clause"),
        word: Word::Succeeds,
        input: || "let x: infer T;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2_147_483_648),
                want_name(
                    "'infer' declarations are only permitted in the 'extends' clause of a conditional type.",
                ),
                want_file("item.ts"),
                want_span((1, 8), (1, 15)),
            ]],
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
    check::<SyntaxErrorItem>();
    reading::<SyntaxErrorItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
