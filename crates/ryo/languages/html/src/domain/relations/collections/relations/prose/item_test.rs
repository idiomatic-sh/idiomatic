//! The reader obeys the shared collection contract on an empty document.
use super::ProseItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("code_is_masked_and_the_run_is_one_row"),
        word: Word::Succeeds,
        input: || "<p>Hello <code>x</code> world</p>",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(12),
                want_node(0),
                want_name("Hello world"),
                want_file("item.html"),
                want_span((1, 1), (1, 34)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("inline_markup_is_joined"),
        word: Word::Succeeds,
        input: || "<p>a<b>b</b> c</p>",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(12),
                want_node(0),
                want_name("ab c"),
                want_file("item.html"),
                want_span((1, 1), (1, 19)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_run_of_code_alone_has_no_row"),
        word: Word::Succeeds,
        input: || "<p><code>x</code></p>",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("empty_document_has_no_semantic_facts"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];
fn subject(source: Input) -> Value {
    super::super::super::fixtures::contract::check::<ProseItem>();
    reading::<ProseItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
