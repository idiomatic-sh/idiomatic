//! Canonical typed-reader contract, the rows of one read, and explicit frontend refusal.
use super::super::fixtures::contract::check;
use super::item::LookupScopeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::proposed;
use ryo::languages::contract::{Point, Reading, Want};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("empty module has a complete empty lookup path"),
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
        name: CaseName("a read in a function is looked up in the function, then in the module"),
        word: Word::Succeeds,
        input: || "x = 1\ndef f():\n    return x\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    Want::Node(24),
                    Want::Name("x"),
                    Want::Node(0),
                    Want::Number(1),
                    Want::Name("value"),
                    Want::File("item.py"),
                    Want::Span(
                        Point {
                            line: 3,
                            column: 12,
                        },
                        Point {
                            line: 3,
                            column: 13,
                        },
                    ),
                ],
                &[
                    Want::Node(24),
                    Want::Name("x"),
                    Want::Node(10),
                    Want::Number(0),
                    Want::Name("value"),
                    Want::File("item.py"),
                    Want::Span(
                        Point {
                            line: 3,
                            column: 12,
                        },
                        Point {
                            line: 3,
                            column: 13,
                        },
                    ),
                ],
            ],
            causes: &[],
        }),
    },
];
#[test]
fn succeeds() {
    check::<LookupScopeItem>();
    run(CASES, Word::Succeeds, |input| {
        Ok(proposed::<LookupScopeItem>(input))
    });
}
