//! Canonical typed-reader contract, the rows of one read, and explicit frontend refusal.
use super::super::fixtures::contract::check;
use super::item::BindingResolutionItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::proposed;
use ryo::languages::contract::{Point, Reading, Want};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("empty module has a complete empty resolution"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("malformed source does not prove absence"),
        word: Word::Succeeds,
        input: || "if:\n    pass\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["binding analysis requires complete supported syntax"],
        }),
    },
    Case {
        name: CaseName("a read after an assignment resolves to it in the module"),
        word: Word::Succeeds,
        input: || "x = 1\nx\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                Want::Node(11),
                Want::Node(2),
                Want::Name("bind"),
                Want::Node(0),
                Want::Node(0),
                Want::Number(0),
                Want::Number(1),
                Want::Name("value"),
                Want::File("item.py"),
                Want::Span(
                    Point { line: 2, column: 1 },
                    Point { line: 2, column: 2 },
                ),
            ]],
            causes: &[],
        }),
    },
];
#[test]
fn succeeds() {
    check::<BindingResolutionItem>();
    run(CASES, Word::Succeeds, |input| {
        Ok(proposed::<BindingResolutionItem>(input))
    });
}
