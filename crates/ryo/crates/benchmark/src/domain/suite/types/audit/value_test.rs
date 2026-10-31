//! Proves one value's behaviour.
use super::value::Audit;
use crate::cases::{Case, CaseName, Expect, Word, run};
const CASES: &[Case<Vec<String>, usize, ()>] = &[
    Case {
        name: CaseName("authored subset is retained"),
        word: Word::Succeeds,
        input: || vec!["A".into()],
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("missing rule cannot be selected"),
        word: Word::Refuses,
        input: || vec!["B".into()],
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("empty selection cannot certify"),
        word: Word::Refuses,
        input: Vec::new,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("duplicate selection cannot certify"),
        word: Word::Refuses,
        input: || vec!["A".into(), "A".into()],
        expect: Expect::Error(()),
    },
];
fn subject(rules: Vec<String>) -> Result<usize, ()> {
    let audit = Audit {
        implemented: vec!["A".into()],
        missing: vec!["B".into()],
    };
    assert_eq!(audit.authored_count(), 1);
    audit
        .select(&rules)
        .map(|selected| selected.authored().len())
        .map_err(|_| ())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
