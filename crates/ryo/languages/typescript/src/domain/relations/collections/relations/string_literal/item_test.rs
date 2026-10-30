//! Cases for TypeScript `string_literal` rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::StringLiteralItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_flag, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("strings_are_decoded"),
        word: Word::Succeeds,
        input: || "const x = \"hi\\n\";",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(10),
                want_node(0),
                want_name("hi\n"),
                want_file("item.ts"),
                want_span((1, 11), (1, 17)),
                want_flag(false),
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
    Case {
        name: CaseName("an_import_specifier_is_marked"),
        word: Word::Succeeds,
        input: || "import a from \"./module\";",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(10),
                want_node(0),
                want_name("./module"),
                want_file("item.ts"),
                want_span((1, 15), (1, 25)),
                want_flag(true),
            ]],
            causes: &[],
        }),
    },
];
fn subject(source: Input) -> Value {
    check::<StringLiteralItem>();
    reading::<StringLiteralItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
