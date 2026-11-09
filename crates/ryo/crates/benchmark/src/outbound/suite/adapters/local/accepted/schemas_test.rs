//! What the accepted-differences document accepts and refuses, one row of `CASES` for each input.
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = &'static [u8];
type Value = (usize, usize);
type Failure = ();
/// A written corpus with findings by side is accepted; an unknown side or key is refused.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rules with findings by side are accepted differences"),
        word: Word::Succeeds,
        input: || {
            &br#"{"description":"d","corpus":"c","differences":[{"rule":"B020","reason":"r","findings":[{"kind":"extra","file":"a.py","line":1,"column":2},{"kind":"missing","file":"a.py","line":3,"column":4}]}]}"#[..]
        },
        expect: Expect::Value((1, 2)),
    },
    Case {
        name: CaseName("a side other than missing or extra is refused"),
        word: Word::Refuses,
        input: || {
            &br#"{"description":"d","corpus":"c","differences":[{"rule":"B020","reason":"r","findings":[{"kind":"moved","file":"a.py","line":1,"column":2}]}]}"#[..]
        },
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("an unknown key is refused"),
        word: Word::Refuses,
        input: || &br#"{"description":"d","corpus":"c","differences":[],"counts":{}}"#[..],
        expect: Expect::Error(()),
    },
];
/// Decodes one document and counts its rules and findings.
fn subject(bytes: Input) -> Result<Value, Failure> {
    super::accepted(bytes)
        .map(|accepted| {
            (
                accepted.rules.len(),
                accepted.rules.iter().map(|rule| rule.findings.len()).sum(),
            )
        })
        .map_err(|_refused| ())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
