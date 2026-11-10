//! Canonical typed-reader contract and explicit frontend refusal.
use super::super::fixtures::contract::check;
use super::item::BindingReadItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, proposed};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "builtin association carries applicable numeric availability without a fake declaration",
        ),
        word: Word::Succeeds,
        input: || "len\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_name("len"),
                want_node(0),
                want_node(0),
                want_name("value"),
                want_name("builtin"),
                want_number(3),
                want_number(0),
                want_file(SHOWN),
                want_span((1, 1), (1, 4)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("empty module has complete empty binding facts"),
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
];
#[test]
fn succeeds() {
    check::<BindingReadItem>();
    run(CASES, Word::Succeeds, |input| {
        Ok(proposed::<BindingReadItem>(input))
    });
}
