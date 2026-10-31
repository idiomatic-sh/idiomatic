//! Canonical names and original Unicode source positions are distinct contracts.
use super::super::fixtures::contract::check;
use super::item::IdentifierItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("name expression retains source extent after NFKC"),
        word: Word::Succeeds,
        input: || "𝗲𝘅𝗲𝗰\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_name("exec"),
                want_file(SHOWN),
                want_span((1, 1), (1, 17)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("ASCII identifier has same semantic shape"),
        word: Word::Succeeds,
        input: || "exec\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_name("exec"),
                want_file(SHOWN),
                want_span((1, 1), (1, 5)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("incomplete source is not complete identifier knowledge"),
        word: Word::Succeeds,
        input: || "if:\n pass\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["identifiers require a complete syntax tree"],
        }),
    },
];
fn subject(source: Input) -> Value {
    check::<IdentifierItem>();
    reading::<IdentifierItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
