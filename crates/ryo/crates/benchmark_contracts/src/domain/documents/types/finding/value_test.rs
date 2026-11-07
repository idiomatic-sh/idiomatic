//! Findings order by rule, then file, then span.
use crate::cases::{Case, CaseName, Expect, Word, run};
use super::value::{Finding, FindingOffset};
use crate::domain::documents::types::text::value::{PathText, RuleName};
use std::cmp::Ordering;

type Input = (&'static str, &'static str, u64);
type Value = Ordering;
type Failure = ();
fn finding((rule, file, start): Input) -> Finding {
    Finding {
        rule: RuleName(rule.into()),
        file: PathText(file.into()),
        start: FindingOffset(start),
        end: FindingOffset(start + 1),
    }
}
const CASES: &[Case<(Input, Input), Value, Failure>] = &[
    Case {
        name: CaseName("the rule orders before the file"),
        word: Word::Succeeds,
        input: || (("A", "z.py", 9), ("B", "a.py", 0)),
        expect: Expect::Value(Ordering::Less),
    },
    Case {
        name: CaseName("the file orders before the span"),
        word: Word::Succeeds,
        input: || (("A", "b.py", 0), ("A", "a.py", 9)),
        expect: Expect::Value(Ordering::Greater),
    },
    Case {
        name: CaseName("equal findings are equal"),
        word: Word::Succeeds,
        input: || (("A", "a.py", 3), ("A", "a.py", 3)),
        expect: Expect::Value(Ordering::Equal),
    },
];
fn subject((left, right): (Input, Input)) -> Result<Value, Failure> {
    Ok(finding(left).cmp(&finding(right)))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
