//! Canonical typed-reader contract and the rows of one annotation.
use super::super::fixtures::contract::check;
use super::item::AnnotationEvaluationItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, proposed};
use ryo::languages::contract::{Reading, want_file, want_name, want_node};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("empty module has no annotation"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading { rows: &[], causes: &[] }),
    },
    Case {
        name: CaseName("a parameter annotation and its token at the default target, which defers annotations natively"),
        word: Word::Succeeds,
        input: || "def f(a: int): pass\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[want_node(14), want_node(14), want_name("deferred"), want_name("lazy"), want_file(SHOWN)],
                &[want_node(15), want_node(14), want_name("deferred"), want_name("lazy"), want_file(SHOWN)],
            ],
            causes: &[],
        }),
    },
];
#[test]
fn succeeds() {
    check::<AnnotationEvaluationItem>();
    run(CASES, Word::Succeeds, |input| Ok(proposed::<AnnotationEvaluationItem>(input)));
}
